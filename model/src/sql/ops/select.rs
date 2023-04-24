//! Compilation of select from high-level GraphQL types into low-level SQL types.

use super::{
    super::db::{Connection, Row, Select, SelectColumn, SelectExt},
    column_name, scalar_to_value, table_name, value_to_scalar, Error,
};
use crate::graphql::type_system::{self as gql, ResourcePredicate, ScalarPredicate, Type};
use take_mut::take;

/// Search for items of resource `T` matching `filter`.
pub async fn execute<C: Connection, T: gql::Resource>(
    conn: &C,
    filter: Option<T::Predicate>,
) -> Result<Vec<T>, Error> {
    let table = table_name::<T>();
    let mut query = conn.select(&[SelectColumn::All], &table);
    if let Some(predicate) = filter {
        query = compile_predicate::<_, T>(query, predicate);
    }
    let rows = query.many().await.map_err(Error::sql)?;
    rows.iter().map(parse_row).collect()
}

/// Compiler to turn a scalar predicate into a condition which is part of a `WHERE` clause.
struct ScalarWhereCondition<Q> {
    column: String,
    query: Q,
}

impl<Q: Select, T: gql::Scalar> gql::ScalarPredicateCompiler<T> for ScalarWhereCondition<Q> {
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

/// Compiler to turn a predicate into a condition which is part of a `WHERE` clause.
struct WhereCondition<Q, T: gql::Type> {
    column: String,
    query: Q,
    predicate: T::Predicate,
}

impl<Q: Select, T: gql::Type> gql::Visitor<T> for WhereCondition<Q, T> {
    type Output = Q;

    fn resource(self) -> Q
    where
        T: gql::Resource,
    {
        unimplemented!("relations")
    }

    fn scalar(self) -> Self::Output
    where
        T: gql::Scalar,
    {
        self.predicate.compile(ScalarWhereCondition {
            column: self.column,
            query: self.query,
        })
    }
}

/// Compile a predicate on a resource into a `WHERE` clause on a query of that table.
fn compile_predicate<Q: Select, T: gql::Resource>(query: Q, pred: T::ResourcePredicate) -> Q {
    struct Visitor<Q, T: gql::Resource> {
        query: Q,
        pred: T::ResourcePredicate,
    }

    impl<Q: Select, T: gql::Resource> gql::ResourceVisitor<T> for Visitor<Q, T> {
        type Output = Q;

        fn visit_field_in_place<F: gql::Field<Resource = T>>(&mut self) {
            if let Some(sub_pred) = self.pred.take::<F>() {
                take(&mut self.query, |query| {
                    F::Type::describe(WhereCondition {
                        column: column_name::<F>(),
                        query,
                        predicate: sub_pred,
                    })
                });
            }
        }

        fn visit_plural_field_in_place<F: gql::PluralField<Resource = T>>(&mut self) {
            unimplemented!("plural fields")
        }

        fn end(self) -> Q {
            self.query
        }
    }

    T::describe_resource(Visitor { query, pred })
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
        sql::db::{mock, SchemaColumn, Type, Value},
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
            array![SchemaColumn;
                SchemaColumn::new("field1", Type::Int4),
                SchemaColumn::new("field2", Type::Text),
            ],
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
