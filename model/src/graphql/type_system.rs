//! Type system for a relational GraphQL API.
//!
//! A relational GraphQL API is one which allows the client to filter results based on complex
//! queries that involve relations between nodes of the subgraph being queried. The queries are
//! solved by compiling the expressive GraphQL query into a backend which can efficiently solve
//! relational queries, like an RDBMS.
//!
//! This module defines the types of objects that can appear in such an API along with a
//! backend-agnostic interface for compiling them into low-level queries. Any API which follows this
//! type system can be implemented by any backend which implements the relevant compilation traits.
//! This is useful for testing (substituting a mock backend for the production one) and for avoiding
//! lock-in, because it makes it easy to switch an existing application from one backend to another.
//!
//! At a high-level the type system consists of the following notions:
//! * Every entity in the application's data model has a [`Type`], including scalars (the leaves of
//!   the graph).
//! * Every [`Type`] has a [`Predicate`], a filter that can be applied to objects of that type in
//!   order to prune query output.
//! * A [`Scalar`] is a subtype of [`Type`] which corresponds to a GraphQL scalar. These are the
//!   primitive types, the leaves of a graph. There is a fixed set of supported scalar types
//!   provided by this module, and these cannot be extended.
//! * A [`Resource`] is a subtype of [`Type`] which corresponds to a GraphQL object type or a table
//!   in a relational database. Unlike a [`Scalar`], whose only observable property is its value, a
//!   [`Resource`] can have many named properties of [`Scalar`] type as well as relations to other
//!   [`Resource`] types. These relations can be singular (one-to-one or many-to-one) or plural
//!   (one-to-many or many-to-many). They can be used in queries to easily select objects which
//!   related to a selected resource and to prune results based on a property of a relationship
//!   between resource.

use super::async_graphql as gql;
use derive_more::Display;
use is_type::Is;
use sealed::sealed;
use std::error::Error;
use std::fmt::Display;

pub use resource::*;
pub use scalar::*;

/// The base type of the whole GraphQL type system.
///
/// All objects in a relational GraphQL API, from scalars to copmlext object types, have a type
/// which is a subtype of [`Type`], and this trait describes the functionality which is common to
/// every entity in an API.
pub trait Type: Clone + gql::OutputType {
    /// The name of this type.
    ///
    /// This will usually be the stringified name of the type implementing this trait.
    const NAME: &'static str;

    /// The plural form of [`Self::NAME`].
    const PLURAL_NAME: &'static str;

    /// A boolean predciate on objects of this type.
    type Predicate: Predicate<Self>;

    /// A predicate on collections of objects of this type.
    type PluralPredicate: PluralPredicate<Self>;

    /// Build an object of this type using a builder supplied by the backend.
    ///
    /// This is used to reconstruct an object from a backend-specific query result.
    fn build<B: Builder<Self>>(builder: B) -> Result<Self, B::Error>;
}

/// A boolean predicate on a [`Type`] `T`.
pub trait Predicate<T: Type>: gql::InputType {
    /// Compile this predicate into a form which the backend can execute.
    ///
    /// When a backend data source executes a GraphQL query, it must compile each predicate in the
    /// query into a form which can be applied to data in the backend's particular datda model. The
    /// backend implementation will construct a [`PredicateCompiler`] which is specific to that
    /// backend and pass it to [`compile`](Self::compile). The predicate will use the
    /// backend-agnostic [`PredicateCompiler`] to describe the structure of this predicate and
    /// instruct the backend on how to compile it.
    fn compile<C: PredicateCompiler<T>>(self, compiler: C) -> C::Result;
}

/// A generic interface to a backend-specific predicate compiler.
///
/// A [`Predicate`] can use this interface to instruct an arbitrary backend on how to compile it
/// into a backend-specific format.
pub trait PredicateCompiler<T: Type> {
    /// The backend-specific compilation result.
    type Result;

    /// A compiler specifically for [`ResourcePredicate`]s.
    type Resource: ResourcePredicateCompiler<T, Result = Self::Result>
    where
        T: Resource;

    /// A compiler specifically for [`ScalarPredicate`]s.
    type Scalar: ScalarPredicateCompiler<T, Result = Self::Result>
    where
        T: Scalar;

    /// Compile this predicate as a [`ResourcePredicate`].
    fn resource(self) -> Self::Resource
    where
        T: Resource;

    /// Compile this predicate as a [`ScalarPredicate`].
    fn scalar(self) -> Self::Scalar
    where
        T: Scalar;
}

/// The type of a collection of items of a given [`Type`].
pub trait PluralType {
    /// The type of an item in this collection.
    type Singular: Type;
}

/// A boolean predicate on a [`PluralType`] consisting of items of type `T`.
pub trait PluralPredicate<T: Type>: gql::InputType {
    /// Compile this predicate into a form which the backend can execute.
    ///
    /// When a backend data source executes a GraphQL query, it must compile each predicate in the
    /// query into a form which can be applied to data in the backend's particular datda model. The
    /// backend implementation will construct a [`PluralPredicateCompiler`] which is specific to
    /// that backend and pass it to [`compile`](Self::compile). The predicate will use the
    /// backend-agnostic [`PluralPredicateCompiler`] to describe the structure of this predicate and
    /// instruct the backend on how to compile it.
    fn compile<C: PluralPredicateCompiler<T>>(self, compiler: C) -> C::Result;
}

/// A generic interface to a backend-specific plural predicate compiler.
///
/// A [`PluralPredicate`] can use this interface to instruct an arbitrary backend on how to compile
/// it into a backend-specific format.
pub trait PluralPredicateCompiler<T: Type> {
    /// The backend-specific compilation result.
    type Result;

    /// A predicate which requires at least `min` objects in the collection to match `pred`.
    fn at_least(self, min: usize, pred: T::Predicate) -> Self::Result;

    /// A predicate which requires at most `max` objects in the collection to match `pred`.
    fn at_most(self, max: usize, pred: T::Predicate) -> Self::Result;
}

/// An error encountered while reconstructing a GraphQL [`Type`] from query results.
pub trait BuildError: Error + Sized {
    /// Create an error with the given message.
    ///
    /// The error will indicate that it occured while trying to reconstruct an object of type `T`.
    fn custom<T: Type>(err: impl Display) -> Self;

    /// Create an error with the given message.
    ///
    /// The error will indicate that it occured while trying to reconstruct a field `F`.
    fn field<F: Field>(err: impl Display) -> Self {
        Self::custom::<F::Resource>(format!("error reconstructing field {}: {err}", F::NAME))
    }
}

/// A backend specific interface to query results, used to reconstruct a [`Type`].
pub trait Builder<T: Type> {
    /// An error encountered while attempting to reconstruct the object.
    type Error: BuildError;

    /// A builder specifically for [`Resource`] types.
    type Resource: ResourceBuilder<T, Error = Self::Error>
    where
        T: Resource;

    /// Build a [`Resource`].
    fn resource(self) -> Self::Resource
    where
        T: Resource;

    /// Reconstruct a [`Scalar`].
    fn scalar(self) -> Result<T, Self::Error>
    where
        T: Scalar;
}

pub mod scalar {
    //! In GraphQL, primitives are called _scalars_.
    //!
    //! Scalars make up the leaves of a graph-oriented data model. They are primitive types like
    //! integers and strings, upon which more complex object types are recursively built.
    //!
    //! Unlike complex [`Resource`] types, scalars do not have properties or relationships to other
    //! objects. They are simple, atomic objects whose only property is their value. Users also
    //! cannot create new scalar types, so backends are able to rely on there being a known, finite
    //! set of scalar types upon which to build their own data model.

    use super::*;

    /// A primitive type in the relational GraphQL type system.
    #[sealed]
    pub trait Scalar:
        Type<Predicate = Self::ScalarPredicate> + gql::InputType + gql::ScalarType
    {
        /// Boolean predicates on this scalar type.
        ///
        /// This is always the same type as [`Predicate`](Type::Predicate), but the alias
        /// [`ScalarPredicate`](ScalarPredicate) has the more expressive trait bound
        /// [`ScalarPredicate`] instead of the generic [`Predicate`].
        type ScalarPredicate: ScalarPredicate<Self>;

        /// Comparison operators for this type of scalar.
        type Cmp: Display;

        /// Perform a type-level pattern match on this scalar.
        ///
        /// This allows backend implementations to use the fact that there is a known, finite set of
        /// supported scalar types. The backend can thus use backend-specific properties of each
        /// supported type without placing backend-specific constraints on this trait itself.
        ///
        /// The given [`ScalarVisitor`] must handle the case where `Self` is any of the supported
        /// scalar types.
        fn visit<V: ScalarVisitor<Self>>(visitor: V) -> V::Output;
    }

    /// This trait proves that a scalar is one of the supported scalar types.
    ///
    /// In order to implement a backend, it is often essential to have a finite, known set of scalar
    /// types which the backend must represent. For example, an RDBMS backend will need to map
    /// scalar types to the primitive types supported by the database (e.g. `integer`, `text`,
    /// etc.). This trait proves that, given a bound `T: Scalar`, `T` is one of the following
    /// supported scalar types:
    /// * [`i32`]
    /// * [`i64`]
    /// * [`u32`]
    /// * [`u64`]
    /// * [`String`]
    /// and it proves this in a way that makes this fact usable in Rust code. This is stronger than
    /// the [`macro@sealed`] mechanism (which we also use for scalars) which prevents other
    /// implementations of the [`Scalar`] trait, but which does not allow the Rust compiler to _use_
    /// the information that there are no other implementations.
    ///
    /// This trait allows a backend to pattern match on a [`Scalar`] type, providing functions to
    /// handle only the supported cases. This effectively lists Rust's total pattern matching on
    /// enums to the type level. Just like in each arm of a `match` expression you can use the fact
    /// that the variable being matched is a particular variant of an `enum`, in each method of this
    /// trait you can use the fact that `T` is a certain scalar type, by means of the [`Is`] trait
    /// for type-level equality.
    ///
    /// # Examples
    ///
    /// Suppose we want to check if a scalar value is a default value (0 for integer types and empty
    /// for strings). We could write the following function:
    ///
    /// ```
    /// # use model::graphql::type_system::Scalar;
    /// fn is_default_strict<T: Scalar + Default + PartialEq>(value: &T) -> bool {
    ///     *value == T::default()
    /// }
    /// ```
    ///
    /// But this function has a strict type bound: it is only callable if `T` satisfies the extra
    /// trait bounds [`Default`] and [`PartialEq`]. We will have to propagate these bounds upward
    /// through the call stack, which may be difficult if, for example, this function is being
    /// called on an arbitrary field of a [`Resource`] type. We can't easily write the constraint
    /// that all fields of the [`Resource`] we are working with implement these extra bounds.
    /// Indeed, take the [`ResourceBuilder::field`] method. It must work for all `F: Field`, not
    /// just `F: Field, F::Type: Default + PartialEq`.
    ///
    /// To remove the extra type bounds, we can leverage the fact that a [`Scalar`] must be one of a
    /// few types, all of which happen to implement [`Default`] and [`PartialEq`], by performing a
    /// total pattern match on the type of the scalar:
    ///
    /// ```
    /// # use model::graphql::type_system::scalar::*;
    /// # fn is_default_strict<T: Scalar + Default + PartialEq>(value: &T) -> bool {
    /// #     *value == T::default()
    /// # }
    /// fn is_default<T: Scalar>(value: &T) -> bool {
    ///     struct Visitor<'a, T>(&'a T);
    ///
    ///     impl<'a, T: Scalar> ScalarVisitor<T> for Visitor<'a, T> {
    ///         type Output = bool;
    ///
    ///         fn visit_i32(self) -> bool
    ///         where
    ///             T: I32Scalar
    ///         {
    ///             // Here we can use the fact that `T` is `i32` by casting `self.0` from `&T` to
    ///             // `&i32`, and then use `i32::default()`.
    ///             let value = self.0.into_ref();
    ///             // Now we can call the stricter typed version of the function to handle the
    ///             // rest:
    ///             is_default_strict(value)
    ///         }
    ///
    ///         // The remaining cases are similar:
    ///         fn visit_i64(self) -> bool
    ///         where
    ///             T: I64Scalar
    ///         {
    ///             is_default_strict(self.0.into_ref())
    ///         }
    ///         fn visit_u32(self) -> bool
    ///         where
    ///             T: U32Scalar
    ///         {
    ///             is_default_strict(self.0.into_ref())
    ///         }
    ///         fn visit_u64(self) -> bool
    ///         where
    ///             T: U64Scalar
    ///         {
    ///             is_default_strict(self.0.into_ref())
    ///         }
    ///         fn visit_string(self) -> bool
    ///         where
    ///             T: StringScalar
    ///         {
    ///             is_default_strict(self.0.into_ref())
    ///         }
    ///     }
    ///
    ///     T::visit(Visitor(value))
    /// }
    /// ```
    ///
    /// The more permissive `is_default` requires a lot more code than the stricter
    /// `is_default_strict`, because we cannot treat all cases the same using a single trait bound.
    /// Instead we must explicitly handle each case, proving to the Rust compiler that in each one,
    /// the type in question works in the way we want to use it. Still, this type of pattern
    /// matching may be a crucial function for backend implementations in case they require some
    /// specific extra trait bound on scalar types and don't have a good place to put it.
    pub trait ScalarVisitor<T: Scalar> {
        /// The type of value which is returned by this type-level match.
        type Output;

        /// Handle the case where `T` is [`i32`].
        fn visit_i32(self) -> Self::Output
        where
            T: I32Scalar;

        /// Handle the case where `T` is [`i64`].
        fn visit_i64(self) -> Self::Output
        where
            T: I64Scalar;

        /// Handle the case where `T` is [`u32`].
        fn visit_u32(self) -> Self::Output
        where
            T: U32Scalar;

        /// Handle the case where `T` is [`u64`].
        fn visit_u64(self) -> Self::Output
        where
            T: U64Scalar;

        /// Handle the case where `T` is [`String`].
        fn visit_string(self) -> Self::Output
        where
            T: StringScalar;
    }

    /// A boolean predicate on a scalar type `T`.
    #[sealed]
    pub trait ScalarPredicate<T: Scalar>: Predicate<T> {
        /// Compile this predicate into a form which the backend can execute.
        ///
        /// This performs the same operation as [`compile`](Predicate::compile), but it can be
        /// called directly with a [`ScalarPredicateCompiler`], instead of the more generic
        /// [`PredicateCompiler`]. This is useful when it is known that a [`Predicate`] is actually
        /// a [`ScalarPredicate`].
        ///
        /// It is an invariant that for all `T: ScalarPredicate` and `x: T`,
        /// `x.compile(compiler) == x.compile_scalar_predicate(compiler.scalar())`.
        fn compile_scalar_predicate<C: ScalarPredicateCompiler<T>>(self, compiler: C) -> C::Result;
    }

    /// A generic interface to a backend-specific compiler for predicates on scalars.
    ///
    /// A [`ScalarPredicate`] can use this interface to instruct an arbitrary backend on how to
    /// compile itself into a backend-specific format.
    pub trait ScalarPredicateCompiler<T: Scalar> {
        /// The backend-specific compilation result.
        type Result;

        /// Instruct the backend to compile a comparison-based predicate.
        ///
        /// The predicate will act on scalars of type `T` by comparing a given scalar with a
        /// constant [`Value`], using `op` to do the comparison.
        fn cmp(self, op: T::Cmp, value: Value<T>) -> Self::Result;
    }

    /// A scalar value.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, gql::OneofObject)]
    #[graphql(concrete(name = "I32Value", params(i32)))]
    #[graphql(concrete(name = "I64Value", params(i64)))]
    #[graphql(concrete(name = "U32Value", params(u32)))]
    #[graphql(concrete(name = "U64Value", params(u64)))]
    #[graphql(concrete(name = "StringValue", params(String)))]
    pub enum Value<T: Scalar> {
        /// A literal, constant value.
        Lit(T),
        /// A pattern matching variable.
        Var(usize),
    }

    /// Generate an implementation of the [`build`](Type::build) function for scalars.
    macro_rules! build_scalar {
        () => {
            fn build<B: Builder<Self>>(builder: B) -> Result<Self, B::Error> {
                builder.scalar()
            }
        };
    }

    /// Integral scalars.
    #[sealed]
    pub trait IntScalar: Scalar<Cmp = IntCmpOp> + Copy {}
    #[sealed]
    impl<T: Scalar<Cmp = IntCmpOp> + Copy> IntScalar for T {}

    /// Comparison operators for integral scalar types.
    #[derive(Clone, Copy, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, gql::Enum)]
    pub enum IntCmpOp {
        #[display(fmt = "=")]
        EQ,
        #[display(fmt = "!=")]
        NE,
        #[display(fmt = ">=")]
        GE,
        #[display(fmt = ">")]
        GT,
        #[display(fmt = "<=")]
        LE,
        #[display(fmt = "<")]
        LT,
    }

    macro_rules! int_scalars {
        ($(($t:ty, $visit:ident, $mod:ident)),+ $(,)?) => {
            $(
                pub mod $mod {
                    use super::*;

                    /// A boolean predicate on an integral scalar.
                    #[derive(
                        Clone,
                        Copy,
                        Debug,
                        PartialEq,
                        Eq,
                        PartialOrd,
                        Ord,
                        Hash,
                        gql::OneofObject,
                    )]
                    pub enum Predicate {
                        /// Satisfied if the comparison is true.
                        Cmp(Cmp),
                        /// Satisfied if the integer being filtered matches the given value.
                        Is(Value<$t>),
                    }

                    impl Predicate {
                        /// A predicate which compares integers with `value` using `op`.
                        pub fn cmp(op: IntCmpOp, value: Value<$t>) -> Self {
                            Self::Cmp(Cmp::new(op, value))
                        }

                        /// A predicate which compares integers with `value`.
                        pub fn is(value: Value<$t>) -> Self {
                            Self::Is(value)
                        }
                    }

                    impl super::Predicate<$t> for Predicate {
                        fn compile<C: PredicateCompiler<$t>>(self, compiler: C) -> C::Result {
                            self.compile_scalar_predicate(compiler.scalar())
                        }
                    }

                    #[sealed]
                    impl ScalarPredicate<$t> for Predicate {
                        fn compile_scalar_predicate<C: ScalarPredicateCompiler<$t>>(
                            self,
                            compiler: C,
                        ) -> C::Result {
                            match self {
                                Self::Cmp(cmp) => cmp.compile(compiler),
                                Self::Is(val) => Cmp::new(IntCmpOp::EQ, val).compile(compiler),
                            }
                        }
                    }

                    /// A comparison on an integral scalar.
                    #[derive(
                        Clone,
                        Copy,
                        Debug,
                        PartialEq,
                        Eq,
                        PartialOrd,
                        Ord,
                        Hash,
                        gql::InputObject,
                    )]
                    pub struct Cmp {
                        /// The type of comparison.
                        op: IntCmpOp,
                        /// The value to compare with.
                        value: Value<$t>,
                    }

                    impl Cmp {
                        /// A predicate which compares integers with `value` using `op`.
                        pub fn new(op: IntCmpOp, value: Value<$t>) -> Self {
                            Self { op, value }
                        }

                        /// Compile this copmarison into a backend-specific format using the given
                        /// compiler.
                        pub fn compile<C: ScalarPredicateCompiler<$t>>(
                            &self,
                            compiler: C,
                        ) -> C::Result {
                            compiler.cmp(self.op, self.value)
                        }
                    }

                    /// A predicate which must match a certain quantity of integral scalars.
                    #[derive(
                        Clone,
                        Copy,
                        Debug,
                        PartialEq,
                        Eq,
                        PartialOrd,
                        Ord,
                        Hash,
                        gql::InputObject,
                    )]
                    pub struct QuantifiedPredicate {
                        /// The minimum or maximum number of items which must match.
                        quantity: usize,
                        /// The predicate to match against specific items.
                        predicate: Predicate,
                    }

                    /// A predicate used to filter collections of integral scalars.
                    #[derive(
                        Clone,
                        Copy,
                        Debug,
                        PartialEq,
                        Eq,
                        PartialOrd,
                        Ord,
                        Hash,
                        gql::OneofObject,
                    )]
                    pub enum PluralPredicate {
                        /// Matches if at least some number of items in the collection match a
                        /// predicate.
                        AtLeast(QuantifiedPredicate),
                        /// Matches if at most some number of items in the collection match a
                        /// predicate.
                        AtMost(QuantifiedPredicate),
                        /// Matches if at any items in the collection match a predicate.
                        Any(Predicate),
                        /// Matches if all items in the collection match a predicate.
                        All(Predicate),
                        /// Matches if no items in the collection match a predicate.
                        None(Predicate),
                        /// Matches if the collection includes the specified value.
                        Includes(Value<$t>),
                    }

                    impl super::PluralPredicate<$t> for PluralPredicate {
                        fn compile<C: PluralPredicateCompiler<$t>>(self, _compiler: C) -> C::Result {
                            todo!()
                        }
                    }

                    impl Type for $t {
                        type Predicate = Predicate;
                        type PluralPredicate = PluralPredicate;

                        const NAME: &'static str = stringify!($t);
                        const PLURAL_NAME: &'static str = stringify!($t, s);

                        build_scalar!();
                    }

                    #[sealed]
                    impl Scalar for $t {
                        type ScalarPredicate = Predicate;
                        type Cmp = IntCmpOp;

                        fn visit<V: super::ScalarVisitor<Self>>(visitor: V) -> V::Output {
                            visitor.$visit()
                        }
                    }

                    #[doc = "An integral scalar represented as "]
                    #[doc = stringify!($t)]
                    #[sealed]
                    pub trait Trait: IntScalar<ScalarPredicate = Predicate> + Is<Type = $t>
                    {}
                    #[sealed]
                    impl Trait for $t {}
                }
            )+
        }
    }

    int_scalars! {
        (i32, visit_i32, i32_scalar),
        (i64, visit_i64, i64_scalar),
        (u32, visit_u32, u32_scalar),
        (u64, visit_u64, u64_scalar),
    }

    pub use i32_scalar::Trait as I32Scalar;
    pub use i64_scalar::Trait as I64Scalar;
    pub use u32_scalar::Trait as U32Scalar;
    pub use u64_scalar::Trait as U64Scalar;

    /// A string scalar.
    ///
    /// There is only one kind of string scalar, [`String`], so this trait simply constrains the
    /// implementor to be equal to [`String`] using [`Is`]. This is how the Rust compiler knows that
    /// there is only one string scalar and how backends can exploit that fact.
    #[sealed]
    pub trait StringScalar: Scalar<Cmp = StringCmpOp> + Is<Type = String> {}

    #[sealed]
    impl StringScalar for String {}

    /// A boolean predicate on strings.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, gql::OneofObject)]
    pub enum StringPredicate {
        /// Satisfied if the comparison is true.
        Cmp(StringCmp),
        /// Satisfied if the integer being filtered matches the given value.
        Is(Value<String>),
    }

    impl StringPredicate {
        /// A predicate which compares strings with `value` using `op`.
        pub fn cmp(op: StringCmpOp, value: Value<String>) -> Self {
            Self::Cmp(StringCmp::new(op, value))
        }

        /// A predicate which compares strings with `value`.
        pub fn is(value: Value<String>) -> Self {
            Self::Is(value)
        }
    }

    impl Predicate<String> for StringPredicate {
        fn compile<C: PredicateCompiler<String>>(self, compiler: C) -> C::Result {
            self.compile_scalar_predicate(compiler.scalar())
        }
    }

    #[sealed]
    impl ScalarPredicate<String> for StringPredicate {
        fn compile_scalar_predicate<C: ScalarPredicateCompiler<String>>(
            self,
            compiler: C,
        ) -> C::Result {
            match self {
                Self::Cmp(cmp) => cmp.compile(compiler),
                Self::Is(val) => StringCmp::new(StringCmpOp::EQ, val).compile(compiler),
            }
        }
    }

    /// A comparison on strings.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, gql::InputObject)]
    pub struct StringCmp {
        /// The type of comparison.
        op: StringCmpOp,
        /// The value to compare with.
        value: Value<String>,
    }

    impl StringCmp {
        /// A predicate which compares strings with `value` using `op`.
        pub fn new(op: StringCmpOp, value: Value<String>) -> Self {
            Self { op, value }
        }

        /// Compile this copmarison into a backend-specific format using the given compiler.
        pub fn compile<C: ScalarPredicateCompiler<String>>(self, compiler: C) -> C::Result {
            compiler.cmp(self.op, self.value)
        }
    }

    /// Comparison operators for strings.
    #[derive(Clone, Copy, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, gql::Enum)]
    pub enum StringCmpOp {
        #[display(fmt = "=")]
        EQ,
        #[display(fmt = "!=")]
        NE,
    }

    /// A predicate which must match a certain quantity of strings.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, gql::InputObject)]
    pub struct QuantifiedStringPredicate {
        /// The minimum or maximum number of items which must match.
        quantity: usize,
        /// The predicate to match against specific items.
        predicate: StringPredicate,
    }

    /// A predicate used to filter collections of integral scalars.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, gql::OneofObject)]
    pub enum StringsPredicate {
        /// Matches if at least some number of items in the collection match a
        /// predicate.
        AtLeast(QuantifiedStringPredicate),
        /// Matches if at most some number of items in the collection match a
        /// predicate.
        AtMost(QuantifiedStringPredicate),
        /// Matches if at any items in the collection match a predicate.
        Any(StringPredicate),
        /// Matches if all items in the collection match a predicate.
        All(StringPredicate),
        /// Matches if no items in the collection match a predicate.
        None(StringPredicate),
        /// Matches if the collection includes the specified value.
        Includes(Value<String>),
    }

    impl PluralPredicate<String> for StringsPredicate {
        fn compile<C: PluralPredicateCompiler<String>>(self, _compiler: C) -> C::Result {
            todo!()
        }
    }

    impl Type for String {
        type Predicate = StringPredicate;
        type PluralPredicate = StringsPredicate;

        const NAME: &'static str = "String";
        const PLURAL_NAME: &'static str = "Strings";

        build_scalar!();
    }

    #[sealed]
    impl Scalar for String {
        type ScalarPredicate = StringPredicate;
        type Cmp = StringCmpOp;

        fn visit<V: ScalarVisitor<Self>>(visitor: V) -> V::Output {
            visitor.visit_string()
        }
    }
}

pub mod resource {
    //! Resources are complex types in a relational GraphQL API.
    //!
    //! A resource type is akin to an object type in GraphQL or a table in a relational database. It
    //! has its own fields as well as singular or plural relationships to other resources.
    //! Collections of items of a particular resource can be filter down using a
    //! [`ResourcePredicate`]. Entire collections of items of a given resource type can also be
    //! filtered in or out using a [`PluralPredicate`].
    //!
    //! Users can define their own resources by implementing the [`Resource`] trait and relatives,
    //! or by using the [`macro@Resource`] derive macro.

    use super::*;

    pub use model_derive::Resource;

    /// A complex type in the relational GraphQL type system.
    pub trait Resource: Type<Predicate = Self::ResourcePredicate> {
        /// Boolean predicates on this resource type.
        ///
        /// This is always the same type as [`Predicate`](Type::Predicate), but the alias
        /// [`ResourcePredicate`](ResourcePredicate) has the more expressive trait bound
        /// [`ResourcePredicate`] instead of the generic [`Predicate`].
        type ResourcePredicate: ResourcePredicate<Self>;

        /// Build a resource using a builder supplied by the backend.
        ///
        /// This performs the same operation as [`build`](Type::build), but it can be
        /// called directly with a [`ResourceBuilder`], instead of the more generic
        /// [`Builder`]. This is useful when it is known that a [`Type`] is actually
        /// a [`Resource`].
        ///
        /// It is an invariant that for all `T: Resource`,
        /// `T::build(builder) == T::builder_resource(builder.resource())`.
        fn build_resource<B: ResourceBuilder<Self>>(builder: B) -> Result<Self, B::Error>;
    }

    /// A backend specific interface to query results, used to reconstruct a [`Resource`].
    pub trait ResourceBuilder<T: Resource> {
        /// Error reconstructing the object.
        type Error: BuildError;

        /// Recursively reconstruct the value of a field.
        fn field<F: Field<Resource = T>>(&self) -> Result<F::Type, Self::Error>;
    }

    /// A boolean predicate on a resource type `T`.
    pub trait ResourcePredicate<T: Resource>: Predicate<T> {
        /// Compile this predicate into a form which the backend can execute.
        ///
        /// This performs the same operation as [`compile`](Predicate::compile), but it can be
        /// called directly with a [`ResourcePredicateCompiler`], instead of the more generic
        /// [`PredicateCompiler`]. This is useful when it is known that a [`Predicate`] is actually
        /// a [`ResourcePredicate`].
        ///
        /// It is an invariant that for all `T: ResourcePredicate` and `x: T`,
        /// `x.compile(compiler) == x.compile_resource_predicate(compiler.resource())`.
        fn compile_resource_predicate<C: ResourcePredicateCompiler<T>>(
            self,
            compiler: C,
        ) -> C::Result;
    }

    /// A generic interface to a backend-specific compiler for predicates on a resource type `T`.
    ///
    /// A [`ResourcePredicate`] can use this interface to instruct an arbitrary backend on how to
    /// compile itself into a backend-specific format.
    pub trait ResourcePredicateCompiler<T: Resource> {
        /// The backend-specific compilation result.
        type Result;

        /// Add a predicate on a field of this resource.
        fn field<F: Field<Resource = T>>(self, predicate: <F::Type as Type>::Predicate) -> Self;

        /// Add a predicate on a plural field of this resource.
        fn plural_field<F: PluralField<Resource = T>>(
            self,
            predicate: PluralFieldPredicate<F>,
        ) -> Self;

        /// Finish compilation and extract the result.
        fn end(self) -> Self::Result;
    }

    /// Metadata about a field of a resource.
    pub trait Field {
        /// The type of the field.
        type Type: Type;

        /// The resource that this field belongs to.
        type Resource: Resource;

        /// The name of the field.
        const NAME: &'static str;
    }

    /// Metadata about a plural field of a resource.
    pub trait PluralField {
        /// The type of the field.
        type Type: PluralType;

        /// The resource that this field belongs to.
        type Resource: Resource;

        /// The name of the field.
        const NAME: &'static str;
    }

    /// The [`PluralPredicate`] used to filter a resource by its [`PluralField`] `F`.
    pub type PluralFieldPredicate<F> =
        <<<F as PluralField>::Type as PluralType>::Singular as Type>::PluralPredicate;
}
