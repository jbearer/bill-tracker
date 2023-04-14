//! Instantiation of the data model for GraphQL queries.

pub mod prelude;
pub mod scalars;
pub mod schema;
pub mod traits;
pub mod types;

pub use types::*;

// We would like to define an entire scheme of types, all of which are parametric on the same data
// source type, with this type parameter being instantiated when the schema is created.
// Unfortunately, the design of async-graphql (and GraphQL in general) makes dealing with
// polymorphic types extremely painful. While it should be possible to do what we want with the
// appropriate use of macros, for the time being, we fix a data source and instantiate it as a
// module level type alias, rather than a type parameter on each schema type.
pub use super::DataSource as D;
