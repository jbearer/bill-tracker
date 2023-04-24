//! Instantiation of a GraphQL [`DataSource`](gql::DataSource) for a SQL database.

use super::{db, ops};
use crate::graphql::{
    backend::{self as gql, Many, PageRequest},
    connection::Edge,
    type_system::{PluralType, Resource, Type},
    EmptyFields, ObjectType,
};
use async_trait::async_trait;
use derive_more::From;
use std::fmt::Debug;

/// A data source for the bill tracker API implemented using a PostgreSQL database.
pub type PostgresDataSource = SqlDataSource<db::postgres::Connection>;

/// A data source for the bill tracker API implemented using a SQL database.
#[derive(Clone, Debug, From)]
pub struct SqlDataSource<Db>(Db);

#[async_trait]
impl<Db: 'static + db::Connection + Send + Sync> gql::DataSource for SqlDataSource<Db> {
    type Connection<T: Type, C: ObjectType, E: ObjectType> = SqlConnection<T, C, E>;
    type Error = ops::Error;

    async fn load_page<T: Type, C: ObjectType, E: Clone + ObjectType>(
        &self,
        conn: &Self::Connection<T, C, E>,
        page: PageRequest<usize>,
    ) -> Result<Vec<Edge<usize, T, E>>, Self::Error> {
        Ok(conn.load(page))
    }

    async fn register<T: Resource>(&mut self) -> Result<(), Self::Error> {
        ops::register::execute::<_, T>(&self.0).await
    }

    async fn query<T: Resource>(
        &self,
        filter: Option<T::ResourcePredicate>,
    ) -> Result<Many<Self, T>, Self::Error> {
        let objects = ops::select::execute(&self.0, filter).await?;
        Ok(SqlConnection {
            fields: EmptyFields,
            edges: objects
                .into_iter()
                .map(|obj| SqlEdge {
                    node: obj,
                    fields: EmptyFields,
                })
                .collect(),
        })
    }

    async fn insert<I>(&mut self, resources: I) -> Result<(), Self::Error>
    where
        I: IntoIterator + Send,
        I::IntoIter: Send,
        I::Item: Resource,
    {
        ops::insert::execute(&self.0, resources).await
    }
}

/// A paginated connection to a set of rows.
#[derive(Clone, Debug)]
pub struct SqlConnection<T: Type, C, E: ObjectType> {
    fields: C,
    // For now we just keep all items in the connection in memory. Later we will add pagination.
    edges: Vec<SqlEdge<T, E>>,
}

impl<T: Type, C, E: ObjectType> gql::Connection<C> for SqlConnection<T, C, E> {
    type Cursor = usize;

    fn empty(fields: C) -> Self {
        Self {
            fields,
            edges: Default::default(),
        }
    }

    fn has_next(&self, cursor: &usize) -> bool {
        *cursor + 1 < self.edges.len()
    }

    fn has_previous(&self, cursor: &usize) -> bool {
        *cursor > 0
    }

    fn fields(&self) -> &C {
        &self.fields
    }
}

impl<T: Type, C, E: ObjectType> PluralType for SqlConnection<T, C, E> {
    type Singular = T;
}

impl<T: Type, C, E: Clone + ObjectType> SqlConnection<T, C, E> {
    /// Load a page from a paginated connection.
    pub fn load(&self, page: PageRequest<usize>) -> Vec<Edge<usize, T, E>> {
        let after = page.after.unwrap_or(0);
        let before = page.before.unwrap_or(self.edges.len());
        let edges = &self.edges[after..before];

        let first = page.first.unwrap_or(edges.len());
        let edges = &edges[..first];

        let last = edges.len() - page.last.unwrap_or(edges.len());
        let edges = &edges[last..];

        let offset = after + last;
        edges
            .iter()
            .enumerate()
            .map(|(i, edge)| {
                Edge::with_additional_fields(offset + i, edge.node.clone(), edge.fields.clone())
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct SqlEdge<T, E> {
    node: T,
    fields: E,
}
