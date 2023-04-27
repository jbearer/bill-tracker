//! Derive macros for the `model` crate.

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod graphql;
mod helpers;

/// Derive an implementation of `Resource`, and related items, for a struct.
///
/// This macro will derive an implementation of `Resource` for a struct, along with all of the
/// necessary types to describe the resource's predicate and plural predicate. It will also generate
/// an `async_graphql` `#[Object]` `impl` block with resolvers for each of the structs fields.
///
/// Specifically, the following items are generated:
/// * An `#[Object]` `impl` block with a resolver for each field. For singular fields, the resolver
///   simply returns a reference to the field, which must be an `OutputType`. You can use the
///   [`skip`](#field-attributes) attribute to avoid generating the resolver for fields which are
///   not output types. For plural fields, the resolver is paginated: it takes Relay-style paging
///   arguments and loads the appropriate page of results on demand.
/// * A unit struct for each field of the original struct, containing metadata about the field via
///   the `Field` or `PluralField` trait. These are generated in a nested module called `fields`.
/// * An input type for the struct, which contains only the struct's input fields, with references
///   to other resources replaced by the resource ID.
/// * A _has_ predicate, which is a GraphQL input type allowing the client to apply a filter to any
///   of the struct's fields.
/// * A _singular predicate_ used to filter items of this resource. The predicate is an enum with
///   one variant for the _has_ predicate and, if the struct has a [`primary`](#field-attributes),
///   another variant to filter directly by the primary field.
/// * A _quantified predicate_ which applies to a collection of items of this resource by requiring
///   that a certain number of items in the collection match the _singular predicate_.
/// * A _plural predicate_ which applies to a collection of items of this resource. The plural
///   predicate is an enum which has variants requiring that at least or at most _n_ items match a
///   singular predicate, that any, all, or none items match a singular predicate, and, if the
///   struct has a primary field, that the collection includes a given value of the primary field.
///
/// All of these items are placed in a module with the same visibility as the original struct. The
/// items have the same visibility as the original struct, unless the original struct is private, in
/// which case the generated items are `pub(super)` (so that they are visible one level up from the
/// generated module, in the scope where the original struct was defined). The name of the generated
/// module is, by default, the name of the struct converted to snake_case. This can be changed with
/// the [`module`](#struct-attributes) attribute.
///
/// Documentation (doc comments or the `#[doc = "..."]` attribute) on the struct and its fields is
/// automatically propagated to derived items and will appear in the exported GraphQL schema.
///
/// # Examples
///
/// ## Derive a `Resource`.
/// ```
/// # mod example {
/// use model::graphql::prelude::*;
///
/// /// A new resource.
/// #[derive(Clone, Resource)]
/// struct MyResource {
///     id: Id,
///     /// A singular field.
///     #[resource(primary)]
///     singular: u64,
///     /// A plural field.
///     plural: Many<D, u64>,
///     /// A field that is not exposed to GraphQL.
///     #[resource(skip)]
///     extra: WeirdType,
/// }
///
/// #[derive(Clone, Default)]
/// struct WeirdType;
/// # }
/// ```
///
/// ## Use it in a GraphQL schema.
///
/// ```
/// # mod example {
/// # use model::graphql::prelude::*;
/// # #[derive(Clone, Resource)]
/// # struct MyResource {
/// #     id: Id,
/// #     #[resource(primary)]
/// #     singular: u64,
/// #     plural: Many<D, u64>,
/// #     #[resource(skip)]
/// #     extra: WeirdType,
/// # }
/// # #[derive(Clone, Default)]
/// # struct WeirdType;
/// use model::graphql::{backend::Connection, EmptyFields, EmptyMutation, EmptySubscription};
///
/// struct Query;
///
/// #[Object]
/// impl Query {
///     async fn my_resource(
///         &self,
///         #[graphql(name = "where")] filter: <MyResource as Type>::Predicate,
///     ) -> MyResource {
///         MyResource {
///             id: 0,
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
///         myResource(
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
///         myResource(
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
/// | id            | Use this field as the ID for this resource. Each resource must have exactly one ID field of type `Id`. This attribute can be omitted if the field's type is explicitly `Id` (but the macro is required if, e.g., the type of the field is a type alias). | n/a | yes |
/// | plural        | Mark this as a plural field. By default, any field with type `Many<...>` is considered plural. This attribute can be used to pluralize a field with a different type or type alias. | n/a | no |
/// | plural        | Explicitly set whether this field is plural or not by providing a `bool` literal. This can be used to override implicitly plural types like `Many`. | bool | no |
/// | primary       | Mark this field as primary. The primary field may be used in place of the whole object in GraphQL predicates. A struct can have at most one primary field. | n/a | no |
/// | skip          | Do not include this field in the GraphQL types. The type of the field must implement [`Default`], unless an explicit default initializer is provided (the second form). | n/a | no |
/// | skip          | Skip a field, reconstructing it with the given expression when loading this object. | expr | no |
///
#[proc_macro_derive(Resource, attributes(resource))]
pub fn graphql_resource(input: TokenStream) -> TokenStream {
    graphql::resource::derive(parse_macro_input!(input)).into()
}

/// Derive resolvers for top-level query fields in a GraphQL API.
///
/// This macro generates an `async_graphql` `#[Object]` `impl` block for a struct, which includes
/// resolvers for a number of given resource types. These resources act as the entrypoints to an
/// ontology, the rest of which is implied by the transitive closure of the entrypoint resources'
/// fields.
///
/// The resources to include as entrypoints are given using the `resource` attribute, as in
/// `#[query(resource(entrypoint: ResourceType))]`. `entrypoint` is the name of the GraphQL field
/// which will be generated on the query object. It must be a Rust identifier (but will be converted
/// from snake case to camel case in the GraphQL schema). `ResourceType` is the type of items in the
/// collection returned by `entrypoint`. You may specify multiple entrypoints that resolve to the
/// same resource type as long as they have different names.
///
/// Each entrypoint defined this way will generate a GraphQL field which takes the plural resource
/// inputs (a `where` predicate and Relay paging inputs) and produces a paginated connection of
/// items of the resource type matching the `where` clause.
///
/// The macro also generates a `register` associated function on the query type, with the signature:
///
/// ```ignore
/// pub async fn register<D: DataSource>(db: &mut D) -> Result<(), D::Error>;
/// ```
///
/// This function will traverse the list of resources in the query object and register all of the
/// reachable resource types in the database, effectively creating a relational schema for the
/// ontology defined by this object.
///
/// # Examples
///
/// ```
/// # mod example {
/// use model::graphql::{prelude::*, EmptyMutation, EmptySubscription};
///
/// #[derive(Clone, Resource)]
/// struct MyResource {
///     id: Id,
///     #[resource(primary)]
///     field: u64,
/// }
///
/// #[derive(Query)]
/// #[query(resource(my_resources: MyResource))]
/// struct Query;
///
/// # pub async fn example() {
/// let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
///
/// // Query by field.
/// schema
///     .execute("{
///         myResources(
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
/// | resource      | Resource which should included in the ontology          | field   | yes      |
#[proc_macro_derive(Query, attributes(query))]
pub fn graphql_query(input: TokenStream) -> TokenStream {
    graphql::query::derive(parse_macro_input!(input)).into()
}
