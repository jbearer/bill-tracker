//! Interfaces provided by a backend data source consumed by the GraphQL model.
//!
//! The entrypoint to this system of traits is [`DataSource`], which describes the interface by
//! which the GraphQL API interacts with the backend data provider. This is, in particular, the glue
//! between the GraphQL and SQL models, since the SQL model will implement [`DataSource`] and the
//! GraphQL layer will interact with the SQL layer exclusively through this trait.
//!
//! A number of supporting traits are defined here which can be accessed through the [`DataSource`]
//! trait by means of its associated types.

use super::{
    connection::{CursorType, Edge},
    type_system::{Resource, Type},
    EmptyFields, ObjectType,
};
use async_trait::async_trait;
use std::error::Error;

/// A Relay-style paginated connection to a collection of objects.
///
/// The objects in the collection are of type `T`. Each object in the collection also represents a
/// relationship, or _edge_, between the object which owns the collection and the object in the
/// collection. These edges may have additional fields of type `E`, beyond the fields specified by
/// Relay. The connection itself may also have additional fields of type `C`, beyond the fields
/// specified by Relay.
pub trait Connection<C> {
    /// An index into this collection.
    type Cursor: CursorType + Send + Sync;

    /// An empty connection.
    fn empty(fields: C) -> Self;

    /// Are there more objects after `cursor`?
    fn has_next(&self, cursor: &Self::Cursor) -> bool;
    /// Are there more objects before `cursor`?
    fn has_previous(&self, cursor: &Self::Cursor) -> bool;

    /// Get the additional connection-level fields.
    fn fields(&self) -> &C;
}

/// A source of data which can be served by the GraphQL API.
#[async_trait]
pub trait DataSource {
    /// A Relay-style paginated connection to a collection of objects.
    ///
    /// THe objects in the collection are of type `T`. Each object in the collection also represents
    /// a relationship, or _edge_, between the object which owns the collection and the object in
    /// the collection. These edges may have additional fields of type `E`, beyond the fields
    /// specified by Relay. The connection itself may also have additional fields of type `C`,
    /// beyond the fields specified by Relay.
    type Connection<T: Type, C: ObjectType, E: ObjectType>: Connection<C>;
    /// Errors reported while attempting to load data.
    type Error: Error;

    /// Load a page from a paginated connection.
    async fn load_page<T: Type, C: ObjectType, E: Clone + ObjectType>(
        &self,
        conn: &Self::Connection<T, C, E>,
        page: PageRequest<Cursor<Self, T, C, E>>,
    ) -> Result<Vec<Edge<Cursor<Self, T, C, E>, T, E>>, Self::Error>;

    /// Register the resource `T` in the schema.
    async fn register<T: Resource>(&mut self) -> Result<(), Self::Error>;

    /// Get a paginated stream of items matching `filter`.
    async fn query<T: Resource>(
        &self,
        filter: Option<T::ResourcePredicate>,
    ) -> Result<Many<Self, T>, Self::Error>;

    /// Insert new items into the database.
    async fn insert<I>(&mut self, resources: I) -> Result<(), Self::Error>
    where
        I: IntoIterator + Send,
        I::Item: Resource,
        I::IntoIter: Send;
}

/// A specification of a page to load in a paginated connection.
pub struct PageRequest<Cursor> {
    /// Limit the results to the first N items that otherwise match the request.
    pub first: Option<usize>,
    /// Start the page at the first item after that indicated by this cursor.
    pub after: Option<Cursor>,
    /// Limit the results to the last N items that otherwise match the request.
    pub last: Option<usize>,
    /// Start the page at the first item before that indicated by this cursor.
    pub before: Option<Cursor>,
}

/// A one-to-many or many-to-many relationship to another [`Resource`].
pub type Many<D, T, C = EmptyFields, E = EmptyFields> = <D as DataSource>::Connection<T, C, E>;

/// An index into [`Many`].
pub type Cursor<D, T, C = EmptyFields, E = EmptyFields> =
    <Many<D, T, C, E> as Connection<C>>::Cursor;
