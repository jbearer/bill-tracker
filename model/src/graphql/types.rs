//! Types and traits describing the GraphQL protocol.
//!
//! These types constitute the Rust bindings to the generic GraphQL protocol. They form the
//! foundation upon which our specific GraphQL API is built.
//!
//! Many of the types in this module are reexports from the [async_graphql] crate.

use std::fmt::Debug;

// Re-export `async_graphql` directly as an escape hatch.
pub extern crate async_graphql;

pub use async_graphql::{
    connection, Context, EmptyMutation, EmptySubscription, InputObject, InputType, Object,
    ObjectType, OneofObject, OutputType, Result, Schema, SimpleObject,
};
pub use model_derive::{Class, Query};

/// Placeholder for connection objects (connections or edges) which have no additional fields.
//
// Note: async_graphql defines its own [`EmptyFields`](async_graphql::connection::EmptyFields)
// struct, but inconveniently, it does not implement [`Clone`], so we use our own version.
#[derive(Clone, Copy, Debug, SimpleObject)]
#[graphql(fake)]
pub struct EmptyFields;

/// A class type in the bill tracker GraphQL API.
///
/// A class type is akin to an object type in GraphQL or a table in a relational database. It has
/// its own fields as well as singular or plural relationships to other classes. Collections of
/// items of a particular class type can be filter down using a [`Predicate`](Self::Predicate).
/// Entire collections of items of this class type can also be filtered in or out using a
/// [`PluralPredicate`](Self::PluralPredicate).
pub trait Class: OutputType {
    /// A collection of items of this class.
    type Plural: Plural<Singular = Self>;

    /// A predicate on items of this class.
    type Predicate: InputType + Clone + Debug;

    /// A predicate on whole collections of this class.
    type PluralPredicate: InputType + Clone + Debug;
}

/// A homogenous collection of items of [`Class`] type.
pub trait Plural {
    /// The type of a single item in this collection.
    type Singular: Class<Plural = Self>;
}

/// A [`Scalar`] is a special type of [`Class`] which is represented as a GraphQL scalar.
pub trait Scalar: Clone + Debug + Class + InputType {}
impl<T: Clone + Debug + Class + InputType> Scalar for T {}
