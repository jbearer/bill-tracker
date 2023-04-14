//! Derive macros for the `model` crate.

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod graphql;
mod helpers;

/// Derive an implementation of `Class`, and related items, for a struct.
///
/// This macro will derive an implementation of `Class` for a struct, along with all of the
/// necessary types to describe the class's predicate and plural predicate. It will also generate an
/// `async_graphql` `#[Object]` `impl` block with resolvers for each of the structs fields.
///
/// Specifically, the following items are generated:
/// * An `#[Object]` `impl` block with a resolver for each field. For singular fields, the resolver
///   simply returns a reference to the field, which must be an `OutputType`. You can use the
///   [`skip`](#field-attributes) attribute to avoid generating the resolver for fields which are
///   not output types. For plural fields, the resolver is paginated: it takes Relay-style paging
///   arguments and loads the appropriate page of results on demand.
/// * A _has_ predicate, which is a GraphQL input type allowing the client to apply a filter to any
///   of the struct's fields.
/// * A _singular predicate_ used to filter items of this class. The predicate is an enum with one
///   variant for the _has_ predicate and, if the struct has a [`primary`](#field-attributes),
///   another variant to filter directly by the primary field.
/// * A _quantified predicate_ which applies to a collection of items of this class by requiring
///   that a certain number of items in the collection match the _singular predicate_.
/// * A _plural predicate_ which applies to a collection of items of this class. The plural
///   predicate is an enum which has variants requiring that at least or at most _n_ items match a
///   singular predicate, that any, all, or none items match a singular predicate, and, if the
///   struct has a primary field, that the collection includes a given value of the primary field.
///
/// All of these are generated as public items in a module with the same visibility as the original
/// struct. The name of the module is, by default, the name of the struct converted to snake_case.
/// This can be changed with the [`module`](#struct-attributes) attribute.
///
/// Documentation (doc comments or the `#[doc = "..."]` attribute) on the struct and its fields is
/// automatically propagated to derived items and will appear in the exported GraphQL schema.
///
/// # Examples
///
/// ## Derive a `Class`.
/// ```
/// # mod example {
/// use model::graphql::prelude::*;
/// use model::graphql::{traits::Many, D};
///
/// /// A new class.
/// #[derive(Class)]
/// #[class(plural(MyClasses))]
/// struct MyClass {
///     /// A singular field.
///     #[class(primary)]
///     singular: u64,
///     /// A plural field.
///     plural: Many<D, u64>,
///     /// A field that is not exposed to GraphQL.
///     #[class(skip)]
///     extra: WeirdType,
/// }
///
/// struct WeirdType;
/// # }
/// ```
///
/// ## Generated code.
///
/// ```
/// # mod example {
/// # use model::graphql::prelude::*;
/// # use model::graphql::{traits::Many, D};
/// # struct MyClass {
/// #     singular: u64,
/// #     plural: Many<D, u64>,
/// #     extra: WeirdType,
/// # }
/// # struct WeirdType;
/// mod my_class {
///     use super::*;
///     use model::graphql::{*, connection::Connection, scalars::Value, traits::DataSource};
///
///     #[doc = "A new class."]
///     #[Object]
///     impl MyClass {
///         #[doc = "A singular field."]
///         async fn singular(&self) -> &u64 {
///             &self.singular
///         }
///
///         #[doc = "A plural field."]
///         async fn plural(
///             &self,
///             #[graphql(name = "where")]
///             filter: Option<<<Many<D, u64> as Plural>::Singular as Class>::Predicate>,
///             after: Option<String>,
///             before: Option<String>,
///             first: Option<usize>,
///             last: Option<usize>,
///         ) -> Result<
///             Connection<
///                 <D as DataSource>::Cursor<
///                     <Many<D, u64> as Plural>::Singular, EmptyFields
///                 >,
///                 <Many<D, u64> as Plural>::Singular,
///             >
///         >
///         {
///             // Implementation omitted.
/// #           todo!()
///         }
///     }
///
///     #[doc = "A predicate on fields of MyClass."]
///     #[derive(Clone, Debug, InputObject)]
///     pub struct MyClassHas {
///         singular: Option<<u64 as Class>::Predicate>,
///         plural: Option<<<Many<D, u64> as Plural>::Singular as Class>::PluralPredicate>,
///     }
///
///     #[doc = "A predicate used to filter MyClasses."]
///     #[derive(Clone, Debug, OneofObject)]
///     pub enum MyClassPredicate {
///         /// Filter by fields.
///         Has(MyClassHas),
///         /// Filter by value.
///         Is(<u64 as Class>::Predicate),
///     }
///
///     #[doc = "A predicate which must match a certain quantity of MyClasses."]
///     #[derive(Clone, Debug, InputObject)]
///     pub struct QuantifiedMyClassPredicate {
///         /// The minimum or maximum number of items which must match.
///         quantity: usize,
///         /// The predicate to match against specific items.
///         predicate: MyClassPredicate,
///     }
///
///     #[doc = "A predicate used to filter collections of MyClasses."]
///     #[derive(Clone, Debug, OneofObject)]
///     pub enum MyClassesPredicate {
///         /// Matches if at least some number of items in the collection match a predicate.
///         AtLeast(QuantifiedMyClassPredicate),
///         /// Matches if at most some number of items in the collection match a predicate.
///         AtMost(QuantifiedMyClassPredicate),
///         /// Matches if at any items in the collection match a predicate.
///         Any(MyClassPredicate),
///         /// Matches if all items in the collection match a predicate.
///         All(MyClassPredicate),
///         /// Matches if no items in the collection match a predicate.
///         None(MyClassPredicate),
///         /// Matches if the collection includes the specified value.
///         Includes(Value<u64>),
///     }
///
///     impl Class for MyClass {
///         type Plural = Many<D, Self>;
///         type Predicate = MyClassPredicate;
///         type PluralPredicate = MyClassesPredicate;
///     }
/// }
/// # }
/// ```
///
/// ## Use it in a GraphQL schema.
///
/// ```
/// # mod example {
/// # use model::graphql::prelude::*;
/// # use model::graphql::{traits::Many, D};
/// # #[derive(Class)]
/// # #[class(plural(MyClasses))]
/// # struct MyClass {
/// #     #[class(primary)]
/// #     singular: u64,
/// #     plural: Many<D, u64>,
/// #     #[class(skip)]
/// #     extra: WeirdType,
/// # }
/// # struct WeirdType;
/// use model::graphql::{traits::Connection, EmptyFields, EmptyMutation, EmptySubscription};
///
/// struct Query;
///
/// #[Object]
/// impl Query {
///     async fn my_class(
///         &self,
///         #[graphql(name = "where")] filter: my_class::MyClassPredicate,
///     ) -> MyClass {
///         MyClass {
///             singular: 0,
///             plural: Many::<D, u64>::empty(EmptyFields),
///             extra: WeirdType,
///         }
///     }
/// }
///
/// # pub async fn example() {
/// let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
///
/// // Query by field.
/// schema
///     .execute("{
///         myClass(
///             where: {
///                 has: {
///                     plural: {
///                         any: { is: { lit: 0 } }
///                     }
///                 }
///             }
///         ) {
///             singular
///         }
///     }")
///     .await
///     .into_result()
///     .unwrap();
///
/// // Query by primary field.
/// schema
///     .execute("{
///         myClass(
///             where: {
///                 is: { is: { lit: 0 } }
///             }
///         ) {
///             singular
///         }
///     }")
///     .await
///     .into_result()
///     .unwrap();
/// # }
/// # }
/// # async_std::task::block_on(example::example());
/// ```
///
/// # Struct attributes
///
/// | Attribute     | Description                                             | Arg     | Required |
/// |---------------|---------------------------------------------------------|---------|----------|
/// | plural        | Override the default pluralization of the struct name. The default simply appends an `s`. | ident | no |
/// | module        | Override the default module name for derived items. The name defaults to the snake_case version of the struct name. | ident | no |
///
/// # Field attributes
///
/// | Attribute     | Description                                             | Arg    | Required |
/// |---------------|---------------------------------------------------------|---------|----------|
/// | plural        | Mark this as a plural field. By default, any field with type `Many<...>` is considered plural. This attribute can be used to pluralize a field with a different type or type alias. | n/a | no |
/// | plural        | Explicitly set whether this field is plural or not by providing a `bool` literal. This can be used to override implicitly plural types like `Many`. | bool | no |
/// | primary       | Mark this field as primary. The primary field may be used in place of the whole object in GraphQL predicates. A struct can have at most one primary field. | n/a | no |
/// | skip          | Do not include this field in the GraphQL types.         | n/a    | no       |
///
#[proc_macro_derive(Class, attributes(class))]
pub fn graphql_class(input: TokenStream) -> TokenStream {
    graphql::class::derive(parse_macro_input!(input)).into()
}

/// Derive resolvers for top-level query fields in a GraphQL API.
///
/// This macro generates an `async_graphql` `#[Object]` `impl` block for a struct, which includes
/// resolvers for a number of given class types. These classes act as the entrypoints to an
/// ontology, the rest of which is implied by the transitive closure of the entrypoint classes'
/// fields.
///
/// The classes to include as entrypoints are given using the `class` attribute, as in
/// `#[query(class(entrypoint: ClassType))]`. `entrypoint` is the name of the GraphQL field which
/// will be generated on the query object. It must be a Rust identifier (but will be converted from
/// snake case to camel case in the GraphQL schema). `ClassType` is the type of items in the
/// collection returned by `entrypoint`. You may specify multiple entrypoints that result to the
/// same class type as long as they have different names.
///
/// Each entrypoint defined this way will generate a GraphQL field which takes the plural class
/// inputs (a `where` predicate and Relay paging inputs) and produces a paginated connection of
/// items of the class type matching the `where` clause.
///
/// # Examples
///
/// ```
/// # mod example {
/// use model::graphql::{prelude::*, EmptyMutation, EmptySubscription};
///
/// #[derive(Class)]
/// struct MyClass {
///     #[class(primary)]
///     field: u64,
/// }
///
/// #[derive(Query)]
/// #[query(class(classes: MyClass))]
/// struct Query;
///
/// # pub async fn example() {
/// let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
///
/// // Query by field.
/// schema
///     .execute("{
///         classes(
///             where: {
///                 is: { is: { lit: 0 } }
///             }
///         ) {
///             edges { node { field } }
///         }
///     }")
///     .await
///     .into_result()
///     .unwrap();
/// # }
/// # }
/// ```
///
/// # Struct attributes
///
/// | Attribute     | Description                                             | Arg     | Required |
/// |---------------|---------------------------------------------------------|---------|----------|
/// | class         | Class which should included in the ontology             | field   | yes      |
#[proc_macro_derive(Query, attributes(query))]
pub fn graphql_query(input: TokenStream) -> TokenStream {
    graphql::query::derive(parse_macro_input!(input)).into()
}
