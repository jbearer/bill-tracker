//! Implementation of a relational GraphQL solver using a PostgreSQL backend.

use super::db::{Connection, Query, QueryExt, Row, SelectColumn, Value};
use crate::graphql::type_system::{self as gql, Predicate};
use convert_case::{Case, Casing};
use is_type::Is;
use snafu::Snafu;
use std::fmt::Display;

/// Errors encountered when solving GraphQL queries.
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("{error}"))]
    Sql { error: String },

    #[snafu(display("error parsing resource {resource}: {error}"))]
    ParseResource {
        resource: &'static str,
        error: String,
    },

    #[snafu(display("type mismatch (expected {expected}, got {got})"))]
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },

    #[snafu(display("error building type {ty}: {error}"))]
    Build { ty: &'static str, error: String },

    #[snafu(display("error building field {ty}::{field}: {error}"))]
    BuildField {
        ty: &'static str,
        field: &'static str,
        error: String,
    },
}

impl gql::BuildError for Error {
    fn custom<T: gql::Type>(msg: impl Display) -> Self {
        Self::Build {
            ty: T::NAME,
            error: msg.to_string(),
        }
    }

    fn field<F: gql::Field>(msg: impl Display) -> Self {
        Self::BuildField {
            ty: <F::Resource as gql::Type>::NAME,
            field: F::NAME,
            error: msg.to_string(),
        }
    }
}

impl Error {
    /// An error in the SQL layer.
    pub fn sql(error: impl Display) -> Self {
        Self::Sql {
            error: error.to_string(),
        }
    }
}

/// Search for items of resource `T` matching `filter`.
pub async fn query<C: Connection, T: gql::Resource>(
    conn: &C,
    filter: Option<T::Predicate>,
) -> Result<Vec<T>, Error> {
    let table = table_name::<T>();
    let mut query = conn.select(&[SelectColumn::All], &table);
    if let Some(predicate) = filter {
        query = compile_predicate(query, predicate);
    }
    let rows = query.many().await.map_err(Error::sql)?;
    rows.iter().map(parse_row).collect()
}

/// Compiler to turn a predicate into a condition which is part of a `WHERE` clause.
struct WhereCondition<Q> {
    column: String,
    query: Q,
}

impl<Q: Query, T: gql::Scalar> gql::ScalarPredicateCompiler<T> for WhereCondition<Q> {
    type Result = Q;

    fn cmp(self, op: T::Cmp, value: gql::Value<T>) -> Self::Result {
        match value {
            gql::Value::Lit(x) => {
                self.query
                    .filter(&self.column, op.to_string(), scalar_to_value(x))
            }
            gql::Value::Var(_) => unimplemented!(),
        }
    }
}

impl<Q: Query, T: gql::Resource> gql::ResourcePredicateCompiler<T> for WhereCondition<Q> {
    type Result = Q;

    fn field<F: gql::Field<Resource = T>>(
        self,
        _predicate: <F::Type as gql::Type>::Predicate,
    ) -> Self {
        unimplemented!("relations")
    }

    fn plural_field<F: gql::PluralField<Resource = T>>(
        self,
        _predicate: gql::PluralFieldPredicate<F>,
    ) -> Self {
        unimplemented!("plural relations")
    }

    fn end(self) -> Self::Result {
        unimplemented!("relations")
    }
}

impl<Q: Query, T: gql::Type> gql::PredicateCompiler<T> for WhereCondition<Q> {
    type Result = Q;
    type Resource = Self where T: gql::Resource;
    type Scalar = Self where T: gql::Scalar;

    fn resource(self) -> Self::Resource
    where
        T: gql::Resource,
    {
        self
    }

    fn scalar(self) -> Self::Scalar
    where
        T: gql::Scalar,
    {
        self
    }
}

/// Compiler to turn a predicate on a resource into a `WHERE` clause on a query of that table.
struct WhereClause<Q> {
    query: Q,
}

impl<Q: Query, T: gql::Resource> gql::ResourcePredicateCompiler<T> for WhereClause<Q> {
    type Result = Q;

    fn field<F: gql::Field<Resource = T>>(
        mut self,
        predicate: <F::Type as gql::Type>::Predicate,
    ) -> Self {
        self.query = predicate.compile(WhereCondition {
            column: column_name::<F>(),
            query: self.query,
        });
        self
    }

    fn plural_field<F: gql::PluralField<Resource = T>>(
        self,
        _predicate: gql::PluralFieldPredicate<F>,
    ) -> Self {
        unimplemented!("plural fields")
    }

    fn end(self) -> Self::Result {
        self.query
    }
}

/// Compile a predicate on a resource into a `WHERE` clause on a query of that table.
fn compile_predicate<Q: Query, T: gql::Resource, P: gql::ResourcePredicate<T>>(
    query: Q,
    predicate: P,
) -> Q {
    predicate.compile_resource_predicate(WhereClause { query })
}

/// Builder to help an object reconstruct itself from query results.
struct Builder<'a, R> {
    column: String,
    row: &'a R,
}

impl<'a, R: Row, T: 'a + gql::Type> gql::Builder<T> for Builder<'a, R> {
    type Error = Error;
    type Resource = ResourceBuilder<'a, R> where T: gql::Resource;

    fn resource(self) -> Self::Resource
    where
        T: gql::Resource,
    {
        unimplemented!("joins")
    }

    fn scalar(self) -> Result<T, Error>
    where
        T: gql::Scalar,
    {
        value_to_scalar(self.row.column(&self.column).map_err(Error::sql)?)
    }
}

/// Builder to help a resource object reconstruct itself from query results.
struct ResourceBuilder<'a, R> {
    row: &'a R,
}

impl<'a, R: Row, T: gql::Resource> gql::ResourceBuilder<T> for ResourceBuilder<'a, R> {
    type Error = Error;

    fn field<F: gql::Field<Resource = T>>(&self) -> Result<F::Type, Error>
    where
        F: gql::Field<Resource = T>,
    {
        <F::Type as gql::Type>::build(Builder {
            column: column_name::<F>(),
            row: self.row,
        })
    }
}

/// Convert a row of query results into a resource object.
fn parse_row<R: Row, T: gql::Resource>(row: &R) -> Result<T, Error> {
    T::build_resource(ResourceBuilder { row })
}

/// The name of the table corresponding to the resource `T`.
fn table_name<T: gql::Resource>() -> String {
    to_snake_case(T::PLURAL_NAME)
}

/// The name of the column corresponding to the field `F`.
fn column_name<F: gql::Field>() -> String {
    to_snake_case(F::NAME)
}

/// Convert a string to snake case.
fn to_snake_case(s: &str) -> String {
    use convert_case::Boundary::*;
    s.with_boundaries(&[Hyphen, Underscore, Space, LowerUpper])
        .to_case(Case::Snake)
}

/// Convert a [`Scalar`] to a [`Value`].
fn scalar_to_value<T: gql::Scalar>(val: T) -> Value {
    struct Visitor<T: gql::Scalar>(T);

    impl<T: gql::Scalar> gql::ScalarVisitor<T> for Visitor<T> {
        type Output = Value;

        fn visit_i32(self) -> Self::Output
        where
            T: gql::I32Scalar,
        {
            Value::Int4(self.0.into_val())
        }

        fn visit_i64(self) -> Self::Output
        where
            T: gql::I64Scalar,
        {
            Value::Int8(self.0.into_val())
        }

        fn visit_u32(self) -> Self::Output
        where
            T: gql::U32Scalar,
        {
            Value::UInt4(self.0.into_val())
        }

        fn visit_u64(self) -> Self::Output
        where
            T: gql::U64Scalar,
        {
            Value::UInt8(self.0.into_val())
        }

        fn visit_string(self) -> Self::Output
        where
            T: gql::StringScalar,
        {
            Value::Text(self.0.into_val())
        }
    }

    T::visit(Visitor(val))
}

/// Convert a [`Value`] to a [`Scalar`].
fn value_to_scalar<T: gql::Scalar>(val: Value) -> Result<T, Error> {
    use gql::Scalar;

    struct Visitor(Value);

    /// We parse a scalar from the row differently depending on the desired type.
    impl<T: Scalar> gql::ScalarVisitor<T> for Visitor {
        type Output = Result<T, Error>;

        fn visit_i32(self) -> Self::Output
        where
            T: gql::I32Scalar,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "i32",
                    got: ty,
                }
            })?))
        }

        fn visit_i64(self) -> Self::Output
        where
            T: Is<Type = i64>,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "i64",
                    got: ty,
                }
            })?))
        }
        fn visit_u32(self) -> Self::Output
        where
            T: Is<Type = u32>,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "u32",
                    got: ty,
                }
            })?))
        }
        fn visit_u64(self) -> Self::Output
        where
            T: Is<Type = u64>,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "u64",
                    got: ty,
                }
            })?))
        }
        fn visit_string(self) -> Self::Output
        where
            T: Is<Type = String>,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "string",
                    got: ty,
                }
            })?))
        }
    }

    T::visit(Visitor(val))
}

#[cfg(test)]
mod test {
    use super::{super::db::mock, *};
    use gql::Resource;

    /// A simple test resource with scalar fields.
    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct TestResource {
        field1: i32,
        field2: String,
    }

    #[async_std::test]
    async fn test_resource_predicate() {
        let resources = [
            TestResource {
                field1: 0,
                field2: "foo".into(),
            },
            TestResource {
                field1: 1,
                field2: "bar".into(),
            },
            TestResource {
                field1: 1,
                field2: "baz".into(),
            },
        ];

        let db = mock::Connection::create();
        db.create_table_with_rows(
            "test_resources",
            ["field1", "field2"],
            [
                [
                    resources[0].field1.into(),
                    resources[0].field2.clone().into(),
                ],
                [
                    resources[1].field1.into(),
                    resources[1].field2.clone().into(),
                ],
                [
                    resources[2].field1.into(),
                    resources[2].field2.clone().into(),
                ],
            ],
        )
        .await
        .unwrap();

        // Test a single sub-predicate.
        let predicate = TestResource::has()
            .field1(<i32 as gql::Type>::Predicate::cmp(
                gql::IntCmpOp::EQ,
                gql::Value::Lit(1),
            ))
            .into();
        assert_eq!(
            query::<_, TestResource>(&db, Some(predicate))
                .await
                .unwrap(),
            &resources[1..]
        );

        // Test multiple sub-predicates.
        let predicate = TestResource::has()
            .field1(<i32 as gql::Type>::Predicate::cmp(
                gql::IntCmpOp::EQ,
                gql::Value::Lit(1),
            ))
            .field2(<String as gql::Type>::Predicate::cmp(
                gql::StringCmpOp::NE,
                gql::Value::Lit("baz".into()),
            ))
            .into();
        assert_eq!(
            query::<_, TestResource>(&db, Some(predicate))
                .await
                .unwrap(),
            &resources[1..2],
        );

        // Test no sub-predicates.
        let predicate = TestResource::has().into();
        assert_eq!(
            query::<_, TestResource>(&db, Some(predicate))
                .await
                .unwrap(),
            &resources,
        );
    }
}
