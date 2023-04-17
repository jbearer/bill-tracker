//! Data model for bill-related information.
//!
//! The data model is presented in two equivalent instantiations, one for GraphQL and one for
//! (Postgre)SQL. These are two different ways of viewing the same data.
//!
//! The [graphql] model describes clients' view of the data. It provides an ontology that clients
//! can use to conceptualize the various entities and their relationships as well as an expressive
//! language for querying the data.
//!
//! The [sql] model describes how the data is actually stored in the backend. It gives the server
//! the ability to leverage an RDBMS to efficiently solve GraphQL queries from clients.
//!
//! The two models are kept in sync by automatically generating the SQL model from the GraphQL
//! [schema](graphql::schema). Specifically, we implement a general query planner which is able to
//! translate any GraphQL query into a query against a SQL database, as long as the GraphQL conforms
//! to a [type system](graphql::type_system) for relational ontologies. The SQL implementation is
//! thus completely agnostic to the domain-specific GraphQL schema.

pub mod graphql;
pub mod sql;

/// The [`DataSource`](graphql::backend::DataSource) used as a backend for the GraphQL API.
pub use sql::PostgresDataSource as DataSource;
