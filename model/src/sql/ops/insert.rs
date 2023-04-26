//! Compilation of insert from high-level GraphQL types into low-level SQL types.

use super::{column_name_of_field, scalar_to_value, table_name, Error};
use crate::Array;
use crate::{
    graphql::type_system::{self as gql, Resource, Type},
    sql::db::{Connection, Insert, Value},
};

/// Insert items of resource `T` into the database.
pub async fn execute<C: Connection, T: gql::Resource>(
    conn: &C,
    resources: impl IntoIterator<Item = T>,
) -> Result<(), Error> {
    let table = table_name::<T>();
    let columns = T::field_names().map(column_name_of_field);
    let rows = resources.into_iter().map(build_row);
    conn.insert(&table, columns)
        .rows(rows)
        .execute()
        .await
        .map_err(Error::sql)
}

fn build_row<T: gql::Resource>(resource: T) -> Array<Value, T::NumFields> {
    struct Visitor<'a, T>(&'a T);

    impl<'a, T: gql::Resource> gql::FieldVisitor<T> for Visitor<'a, T> {
        type Output = Value;

        fn visit<F: gql::Field<Resource = T>>(&mut self) -> Self::Output {
            build_row_value::<F>(self.0)
        }
    }

    T::describe_fields(&mut Visitor(&resource))
}

fn build_row_value<F: gql::Field>(resource: &F::Resource) -> Value {
    struct Visitor<'a, F: gql::Field>(&'a F::Resource);

    impl<'a, F: gql::Field> gql::Visitor<F::Type> for Visitor<'a, F> {
        type Output = Value;

        fn resource(self) -> Self::Output
        where
            F::Type: gql::Resource,
        {
            unimplemented!("nested resources")
        }

        fn scalar(self) -> Self::Output
        where
            F::Type: gql::Scalar,
        {
            scalar_to_value(self.0.get::<F>().clone())
        }
    }

    F::Type::describe(Visitor::<F>(resource))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        array,
        sql::{
            db::{mock, SchemaColumn, Type},
            ops,
        },
        typenum::U3,
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

        let resources = [
            TestResource {
                id: 0,
                field1: 0,
                field2: "foo".into(),
            },
            TestResource {
                id: 1,
                field1: 1,
                field2: "bar".into(),
            },
            TestResource {
                id: 2,
                field1: 1,
                field2: "baz".into(),
            },
        ];
        ops::insert::execute(&db, resources.clone()).await.unwrap();
        assert_eq!(
            ops::select::execute::<_, TestResource>(&db, None)
                .await
                .unwrap(),
            resources
        );
    }
}
