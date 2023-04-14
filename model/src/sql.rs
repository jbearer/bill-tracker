//! Instantiation of the data model for a relational database (specifically PostgreSQL).

use crate::graphql::{
    connection::{CursorType, Edge},
    traits::{Connection, Cursor, DataSource, PageRequest},
    Class, ObjectType, OutputType, Plural,
};
use async_trait::async_trait;
use derivative::Derivative;
use snafu::Snafu;
use std::fmt::Debug;
use std::marker::PhantomData;

/// Errors reported by the SQL layer.
#[derive(Clone, Debug, Snafu)]
pub enum PostgreSqlError {}

/// An index into a [`PostgreSqlConnection`].
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = "E: Debug"))]
pub struct PostgreSqlCursor<T, E>(PhantomData<(T, E)>);

impl<T: Send + Sync, E: Send + Sync> CursorType for PostgreSqlCursor<T, E> {
    type Error = PostgreSqlError;

    fn decode_cursor(_s: &str) -> Result<Self, Self::Error> {
        todo!()
    }

    fn encode_cursor(&self) -> String {
        todo!()
    }
}

impl<T: Send + Sync, E: Send + Sync> Cursor for PostgreSqlCursor<T, E> {
    fn has_next(&self) -> bool {
        todo!()
    }

    fn has_previous(&self) -> bool {
        todo!()
    }
}

/// A paginated connection to a set of rows.
#[derive(Derivative)]
#[derivative(Clone(bound = "C: Clone"), Debug(bound = "C: Debug"))]
pub struct PostgreSqlConnection<T, C> {
    fields: C,
    _phantom: PhantomData<T>,
}

impl<T, C> Connection<C> for PostgreSqlConnection<T, C> {
    fn empty(fields: C) -> Self {
        Self {
            fields,
            _phantom: Default::default(),
        }
    }

    fn fields(&self) -> &C {
        &self.fields
    }
}

impl<T: Class<Plural = Self>, C> Plural for PostgreSqlConnection<T, C> {
    type Singular = T;
}

/// A data source for the bill tracker API implemented using a PostgreSQL database.
#[derive(Clone, Debug)]
pub struct PostgreSqlDataSource;

#[async_trait]
impl DataSource for PostgreSqlDataSource {
    type Cursor<T: OutputType, E: ObjectType> = PostgreSqlCursor<T, E>;
    type Connection<T: OutputType, C: ObjectType, E: ObjectType> = PostgreSqlConnection<T, C>;
    type Error = PostgreSqlError;

    async fn load_page<T: OutputType, C: ObjectType, E: ObjectType>(
        &self,
        _conn: &mut Self::Connection<T, C, E>,
        _page: PageRequest<Self::Cursor<T, E>>,
    ) -> Result<Vec<Edge<Self::Cursor<T, E>, T, E>>, Self::Error> {
        todo!()
    }
}
