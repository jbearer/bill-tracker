//! Predicates and operations on GraphQL scalars.

use super::{traits::Many, Class, Scalar, D};
use async_graphql::{Enum, InputObject, InputType, OneofObject};
use derivative::Derivative;

/// A scalar value or variable.
#[derive(Derivative, OneofObject)]
#[graphql(concrete(name = "IntValue", params(i64)))]
#[graphql(concrete(name = "UIntValue", params(u64)))]
#[graphql(concrete(name = "StringValue", params(String)))]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub enum Value<T: Scalar> {
    /// A literal value.re
    Lit(T),
    /// A variable for pattern matching.
    Var(String),
}

/// A predicate which must match a certain quantity of scalars in a collection.
#[derive(Derivative, InputObject)]
#[graphql(concrete(name = "QuantifiedIntPredicate", params(i64)))]
#[graphql(concrete(name = "QuantifiedUIntPredicate", params(u64)))]
#[graphql(concrete(name = "QuantifiedStringPredicate", params(String)))]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub struct Quantified<T: Class> {
    /// The minimum or maximum number of scalars which must match.
    quantity: usize,
    /// The predicate to match against specific scalars.
    predicate: T::Predicate,
}

/// A predicate used to filter collections of scalars.
#[derive(Derivative, OneofObject)]
#[graphql(concrete(name = "IntsPredicate", params(i64)))]
#[graphql(concrete(name = "UIntsPredicate", params(u64)))]
#[graphql(concrete(name = "StringsPredicate", params(String)))]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub enum PluralPredicate<T: Scalar>
where
    Quantified<T>: InputType,
    Value<T>: InputType,
{
    /// Matches if at least some number of items in the collection match a predicate.
    AtLeast(Quantified<T>),
    /// Matches if at most some number of items in the collection match a predicate.
    AtMost(Quantified<T>),
    /// Matches if at any items in the collection match a predicate.
    Any(T::Predicate),
    /// Matches if all items in the collection match a predicate.
    All(T::Predicate),
    /// Matches if no items in the collection match a predicate.
    None(T::Predicate),
    /// Matches if the collection includes the given item.
    Includes(Value<T>),
}

/// A way of comparing integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Enum)]
pub enum IntCmpOp {
    EQ,
    GT,
    GE,
    LT,
    LE,
}

/// A comparison predicate on integers.
#[derive(Derivative, InputObject)]
#[graphql(concrete(name = "IntCmp", params(i64)))]
#[graphql(concrete(name = "UIntCmp", params(u64)))]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub struct IntCmp<I: Scalar>
where
    Value<I>: InputType,
{
    /// The type of comparison.
    op: IntCmpOp,
    /// The value to compare with.
    value: Value<I>,
}

/// A predicate used to filter integers.
#[derive(Derivative, OneofObject)]
#[graphql(concrete(name = "IntPredicate", params(i64)))]
#[graphql(concrete(name = "UIntPredicate", params(u64)))]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub enum IntPredicate<I: Scalar>
where
    Value<I>: InputType,
    IntCmp<I>: InputType,
{
    /// Satisfied if the comparison is true.
    Cmp(IntCmp<I>),
    /// Satisfied if the integer being filter matches the given value.
    Is(Value<I>),
}

macro_rules! int_class {
    ($($t:ty),+) => {
        $(
            impl Class for $t {
                type Plural = Many<D, Self>;
                type Predicate = IntPredicate<Self>;
                type PluralPredicate = PluralPredicate<Self>;
            }
        )+
    }
}

int_class!(i64, u64);

/// A predicate used to filter strings.
#[derive(Clone, Debug, OneofObject)]
pub enum StringPredicate {
    /// Satisfied if the string being filtered matches the given value.
    Is(Value<String>),
}

impl Class for String {
    type Plural = Many<D, Self>;
    type Predicate = StringPredicate;
    type PluralPredicate = PluralPredicate<Self>;
}
