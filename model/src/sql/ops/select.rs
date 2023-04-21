//! Compilation of select from high-level GraphQL types into low-level SQL types.

use super::{
    super::db::{Connection, Row, Select, SelectColumn, SelectExt},
    column_name, scalar_to_value, table_name, value_to_scalar, Error,
};
use crate::graphql::type_system::{self as gql, Predicate};

/// Search for items of resource `T` matching `filter`.
pub async fn execute<C: Connection, T: gql::Resource>(
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

impl<Q: Select, T: gql::Scalar> gql::ScalarPredicateCompiler<T> for WhereCondition<Q> {
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

impl<Q: Select, T: gql::Resource> gql::ResourcePredicateCompiler<T> for WhereCondition<Q> {
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

impl<Q: Select, T: gql::Type> gql::PredicateCompiler<T> for WhereCondition<Q> {
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

impl<Q: Select, T: gql::Resource> gql::ResourcePredicateCompiler<T> for WhereClause<Q> {
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
fn compile_predicate<Q: Select, T: gql::Resource, P: gql::ResourcePredicate<T>>(
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        array,
        sql::db::{mock, Value},
    };
    use generic_array::typenum::U2;
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
        db.create_table_with_rows::<U2>(
            "test_resources",
            array![&str; "field1", "field2"],
            [
                array![Value;
                    Value::from(resources[0].field1),
                    Value::from(resources[0].field2.clone()),
                ],
                array![Value;
                    Value::from(resources[1].field1),
                    Value::from(resources[1].field2.clone()),
                ],
                array![Value;
                    Value::from(resources[2].field1),
                    Value::from(resources[2].field2.clone()),
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
            execute::<_, TestResource>(&db, Some(predicate))
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
            execute::<_, TestResource>(&db, Some(predicate))
                .await
                .unwrap(),
            &resources[1..2],
        );

        // Test no sub-predicates.
        let predicate = TestResource::has().into();
        assert_eq!(
            execute::<_, TestResource>(&db, Some(predicate))
                .await
                .unwrap(),
            &resources,
        );
    }
}
