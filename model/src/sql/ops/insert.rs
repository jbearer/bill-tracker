//! Compilation of insert from high-level GraphQL types into low-level SQL types.

use super::{column_name_of_field, scalar_to_value, table_name, Error};
use crate::Array;
use crate::{
    graphql::type_system::{self as gql, ResourceInput},
    sql::db::{Connection, Insert, Value},
};

/// Insert items of resource `T` into the database.
pub async fn execute<C: Connection, T: gql::Resource>(
    conn: &C,
    inputs: impl IntoIterator<Item = T::ResourceInput>,
) -> Result<(), Error> {
    let table = table_name::<T>();
    let columns = T::input_field_names().map(column_name_of_field);
    let rows = inputs.into_iter().map(build_row::<T>);
    conn.insert(&table, columns)
        .rows(rows)
        .execute()
        .await
        .map_err(Error::sql)
}

fn build_row<T: gql::Resource>(input: T::ResourceInput) -> Array<Value, T::NumInputFields> {
    struct Visitor<'a, T: gql::Resource>(&'a T::ResourceInput);

    impl<'a, T: gql::Resource> gql::InputFieldVisitor<T> for Visitor<'a, T> {
        type Output = Value;

        fn visit<F: gql::InputField<Resource = T>>(&mut self) -> Self::Output {
            scalar_to_value(self.0.get::<F>().clone())
        }
    }

    T::describe_input_fields(&mut Visitor(&input))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        array, init_logging,
        sql::{
            db::{mock, SchemaColumn, Type},
            ops,
        },
        typenum::{U2, U3},
    };
    use gql::{Id, Resource};

    /// A simple test resource with scalar fields.
    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct TestResource {
        id: Id,
        field1: i32,
        field2: String,
    }

    #[async_std::test]
    async fn test_round_trip_no_relations() {
        init_logging();

        let db = mock::Connection::create();
        db.create_table::<U3>(
            "test_resources",
            array![SchemaColumn;
                SchemaColumn::new("id", Type::Serial),
                SchemaColumn::new("field1", Type::Int4),
                SchemaColumn::new("field2", Type::Text),
            ],
        )
        .await
        .unwrap();

        ops::insert::execute::<_, TestResource>(
            &db,
            [
                test_resource::TestResourceInput {
                    field1: 0,
                    field2: "foo".into(),
                },
                test_resource::TestResourceInput {
                    field1: 1,
                    field2: "bar".into(),
                },
                test_resource::TestResourceInput {
                    field1: 1,
                    field2: "baz".into(),
                },
            ],
        )
        .await
        .unwrap();
        assert_eq!(
            ops::select::execute::<_, TestResource>(&db, None)
                .await
                .unwrap(),
            [
                TestResource {
                    id: 1,
                    field1: 0,
                    field2: "foo".into(),
                },
                TestResource {
                    id: 2,
                    field1: 1,
                    field2: "bar".into(),
                },
                TestResource {
                    id: 3,
                    field1: 1,
                    field2: "baz".into(),
                },
            ]
        );
    }

    /// A resource that owns another resource (a one-one or one-many relationship).
    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct Owner {
        id: Id,
        owned: TestResource,
    }

    #[async_std::test]
    async fn test_insert_owner() {
        init_logging();

        let db = mock::Connection::create();
        db.create_table::<U3>(
            "test_resources",
            array![SchemaColumn;
                SchemaColumn::new("id", Type::Serial),
                SchemaColumn::new("field1", Type::Int4),
                SchemaColumn::new("field2", Type::Text),
            ],
        )
        .await
        .unwrap();
        db.create_table::<U2>(
            "owners",
            array![SchemaColumn;
                SchemaColumn::new("id", Type::Serial),
                SchemaColumn::new("owned", Type::Int4),
            ],
        )
        .await
        .unwrap();

        // First we have to insert something to own.
        ops::insert::execute::<_, TestResource>(
            &db,
            [test_resource::TestResourceInput {
                field1: 0,
                field2: "foo".into(),
            }],
        )
        .await
        .unwrap();

        // Now insert something that owns the first resource.
        ops::insert::execute::<_, Owner>(&db, [owner::OwnerInput { owned: 1 }])
            .await
            .unwrap();

        // Read it back.
        assert_eq!(
            ops::select::execute::<_, Owner>(&db, None).await.unwrap(),
            [Owner {
                id: 1,
                owned: TestResource {
                    id: 1,
                    field1: 0,
                    field2: "foo".into()
                }
            }]
        );
    }
}
