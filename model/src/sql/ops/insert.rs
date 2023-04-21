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
        type Resource = NestedResourceVisitor where F::Type: gql::Resource;

        fn resource(self) -> Self::Resource
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

    struct NestedResourceVisitor;

    impl<T: gql::Resource> gql::ResourceVisitor<T> for NestedResourceVisitor {
        type Output = Value;

        fn visit_field_in_place<F: gql::Field<Resource = T>>(&mut self) {}

        fn visit_plural_field_in_place<F: gql::PluralField<Resource = T>>(&mut self) {}

        fn end(self) -> Self::Output {
            unimplemented!("nested resources")
        }
    }

    F::Type::describe(Visitor::<F>(resource))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        array,
        sql::{db::mock, ops},
        typenum::U2,
    };
    use gql::Resource;

    /// A simple test resource with scalar fields.
    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct TestResource {
        field1: i32,
        field2: String,
    }

    #[async_std::test]
    async fn test_round_trip_no_relations() {
        let db = mock::Connection::create();
        db.create_table::<U2>("test_resources", array![&str; "field1", "field2"])
            .await
            .unwrap();

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
        ops::insert::execute(&db, resources.clone()).await.unwrap();
        assert_eq!(
            ops::select::execute::<_, TestResource>(&db, None)
                .await
                .unwrap(),
            resources
        );
    }
}
