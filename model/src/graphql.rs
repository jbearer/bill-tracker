//! Instantiation of the data model for GraphQL queries.

pub mod backend;
pub mod prelude;
pub mod schema;
pub mod type_system;

// Re-export commonly used `async_graphql` types.
pub use async_graphql::{
    connection, Context, EmptyMutation, EmptySubscription, InputObject, InputType, Object,
    ObjectType, OneofObject, OutputType, Result, Schema, SimpleObject,
};
pub use model_derive::Query;

// Re-export `async_graphql` directly as an escape hatch.
pub extern crate async_graphql;

/// Placeholder for connection objects (connections or edges) which have no additional fields.
//
// Note: async_graphql defines its own [`EmptyFields`](async_graphql::connection::EmptyFields)
// struct, but inconveniently, it does not implement [`Clone`], so we use our own version.
#[derive(Clone, Copy, Debug, SimpleObject)]
#[graphql(fake)]
pub struct EmptyFields;

// We would like to define an entire scheme of types, all of which are parametric on the same data
// source type, with this type parameter being instantiated when the schema is created.
// Unfortunately, the design of async-graphql (and GraphQL in general) makes dealing with
// polymorphic types extremely painful. While it should be possible to do what we want with the
// appropriate use of macros, for the time being, we fix a data source and instantiate it as a
// module level type alias, rather than a type parameter on each schema type.
pub use super::DataSource as D;
