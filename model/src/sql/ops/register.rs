//! Compilation of resource registration from high-level GraphQL types into low-level SQL types.

use super::{column_name, lower_scalar_type, table_name, Error};
use crate::{
    graphql::type_system::{self as gql, Type as _},
    sql::db::{Connection, CreateTable, SchemaColumn, Type},
};
use futures::future::{try_join_all, BoxFuture, FutureExt, TryFutureExt};
use std::borrow::Cow;
use std::collections::hash_map::{Entry, HashMap};

/// Register a resource `T` in the database schema.
pub async fn execute<C: Connection, T: gql::Resource>(conn: &C) -> Result<(), Error> {
    let mut dependencies = Dependencies::default();
    register_resource::<C, T>(conn, &mut dependencies);
    try_join_all(dependencies.into_values().flatten()).await?;
    Ok(())
}

fn register_resource<'a, C: Connection, T: gql::Resource>(
    conn: &'a C,
    dependencies: &mut Dependencies<'a>,
) {
    let table = table_name::<T>();
    match dependencies.entry(table.clone()) {
        Entry::Occupied(_) => {
            // If this table is already in the list of tables to register, we have nothing more to
            // do.
            return;
        }
        Entry::Vacant(e) => {
            // Insert a placeholder here so if this type references itself recursively, we won't
            // recursively try to register the same type again.
            e.insert(None);
        }
    }

    let fields = T::describe_fields(&mut ColumnBuilder { conn, dependencies });

    // A plural field is implemented as a foreign key on another table referencing this one, so for
    // this table, we have nothing to do. But we still have to traverse the referenced type and make
    // sure all the appropriate tables are created.
    T::describe_plural_fields(&mut ColumnTraverser { conn, dependencies });

    // Create a future to register this table.
    let fut = conn
        .create_table(Cow::Owned(table.clone()), fields)
        .execute()
        .map_err(Error::sql)
        .boxed();
    dependencies.insert(table, Some(fut));
}

struct ColumnBuilder<'a, 'd, C> {
    conn: &'a C,
    dependencies: &'d mut Dependencies<'a>,
}

impl<'a, 'd, C: Connection, T: gql::Resource> gql::FieldVisitor<T> for ColumnBuilder<'a, 'd, C> {
    type Output = SchemaColumn<'a>;

    fn visit<F: gql::Field<Resource = T>>(&mut self) -> Self::Output {
        struct Visitor<'a, 'd, C> {
            conn: &'a C,
            column_name: String,
            dependencies: &'d mut Dependencies<'a>,
        }

        impl<'a, 'd, C: Connection, T: gql::Type> gql::Visitor<T> for Visitor<'a, 'd, C> {
            type Output = SchemaColumn<'a>;

            fn resource(self) -> Self::Output
            where
                T: gql::Resource,
            {
                // If the field is a reference to another table, make sure that table is registered.
                register_resource::<C, T>(self.conn, self.dependencies);

                // Add the corresponding ID as a foreign key on this table.
                SchemaColumn::new(Cow::Owned(self.column_name), Type::Int8)
            }

            fn scalar(self) -> Self::Output
            where
                T: gql::Scalar,
            {
                // If the field is a scalar, just create a column of the corresponding type.
                SchemaColumn::new(Cow::Owned(self.column_name), lower_scalar_type::<T>())
            }
        }

        F::Type::describe(Visitor {
            conn: self.conn,
            dependencies: self.dependencies,
            column_name: column_name::<F>(),
        })
    }
}

struct ColumnTraverser<'a, 'd, C> {
    conn: &'a C,
    dependencies: &'d mut Dependencies<'a>,
}

impl<'a, 'd, C: Connection, T: gql::Resource> gql::PluralFieldVisitor<T>
    for ColumnTraverser<'a, 'd, C>
{
    type Output = ();

    fn visit<F: gql::PluralField<Resource = T>>(&mut self) -> Self::Output {
        struct Visitor<'a, 'd, C> {
            conn: &'a C,
            dependencies: &'d mut Dependencies<'a>,
        }

        impl<'a, 'd, C: Connection, T: gql::Type> gql::Visitor<T> for Visitor<'a, 'd, C> {
            type Output = ();

            fn resource(self) -> Self::Output
            where
                T: gql::Resource,
            {
                // If the field is a reference to another table, make sure that table is registered.
                register_resource::<C, T>(self.conn, self.dependencies);
            }

            fn scalar(self) -> Self::Output
            where
                T: gql::Scalar,
            {
            }
        }

        <F::Type as gql::PluralType>::Singular::describe(Visitor {
            conn: self.conn,
            dependencies: self.dependencies,
        })
    }
}

// Map from table names to futures creating those tables.
type Dependencies<'a> = HashMap<String, Option<BoxFuture<'a, Result<(), Error>>>>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::sql::db::mock;
    use gql::Resource;

    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct Simple {
        int_field: i32,
        text_field: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct OneToOne {
        simple: Simple,
    }

    #[async_std::test]
    async fn test_simple() {
        let db = mock::Connection::create();
        execute::<_, Simple>(&db).await.unwrap();
        assert_eq!(
            db.schema().await["simples"],
            [
                SchemaColumn::new("int_field", Type::Int4),
                SchemaColumn::new("text_field", Type::Text)
            ]
        );
    }

    #[async_std::test]
    async fn test_one_to_one() {
        let db = mock::Connection::create();
        execute::<_, OneToOne>(&db).await.unwrap();
        let schema = db.schema().await;

        // Ensure the dependency table `simples` was created.
        assert_eq!(
            schema["simples"],
            [
                SchemaColumn::new("int_field", Type::Int4),
                SchemaColumn::new("text_field", Type::Text)
            ]
        );

        // Check the table with the relation, implemented as a foreign key.
        assert_eq!(
            schema["one_to_ones"],
            [SchemaColumn::new("simple", Type::Int8),]
        );
    }
}
