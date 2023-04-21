//! Compilation of high-level GraphQL operations into low-level SQL operations.

use super::db::Value;
use crate::graphql::type_system as gql;
use convert_case::{Case, Casing};
use is_type::Is;
use snafu::Snafu;
use std::fmt::Display;

pub mod insert;
pub mod select;

/// Errors encountered when executing GraphQL operations.
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("{error}"))]
    Sql { error: String },

    #[snafu(display("error parsing resource {resource}: {error}"))]
    ParseResource {
        resource: &'static str,
        error: String,
    },

    #[snafu(display("type mismatch (expected {expected}, got {got})"))]
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },

    #[snafu(display("error building type {ty}: {error}"))]
    Build { ty: &'static str, error: String },

    #[snafu(display("error building field {ty}::{field}: {error}"))]
    BuildField {
        ty: &'static str,
        field: &'static str,
        error: String,
    },
}

impl gql::BuildError for Error {
    fn custom<T: gql::Type>(msg: impl Display) -> Self {
        Self::Build {
            ty: T::NAME,
            error: msg.to_string(),
        }
    }

    fn field<F: gql::Field>(msg: impl Display) -> Self {
        Self::BuildField {
            ty: <F::Resource as gql::Type>::NAME,
            field: F::NAME,
            error: msg.to_string(),
        }
    }
}

impl Error {
    /// An error in the SQL layer.
    pub fn sql(error: impl Display) -> Self {
        Self::Sql {
            error: error.to_string(),
        }
    }
}

/// The name of the table corresponding to the resource `T`.
fn table_name<T: gql::Resource>() -> String {
    to_snake_case(T::PLURAL_NAME)
}

/// The name of the column corresponding to the field `F`.
fn column_name<F: gql::Field>() -> String {
    column_name_of_field(F::NAME)
}

/// The name of the column corresponding to the field with name `field_name`.
fn column_name_of_field(field_name: &'static str) -> String {
    to_snake_case(field_name)
}

/// Convert a string to snake case.
fn to_snake_case(s: &str) -> String {
    use convert_case::Boundary::*;
    s.with_boundaries(&[Hyphen, Underscore, Space, LowerUpper])
        .to_case(Case::Snake)
}

/// Convert a [`Scalar`] to a [`Value`].
fn scalar_to_value<T: gql::Scalar>(val: T) -> Value {
    struct Visitor<T: gql::Scalar>(T);

    impl<T: gql::Scalar> gql::ScalarVisitor<T> for Visitor<T> {
        type Output = Value;

        fn visit_i32(self) -> Self::Output
        where
            T: gql::I32Scalar,
        {
            Value::Int4(self.0.into_val())
        }

        fn visit_i64(self) -> Self::Output
        where
            T: gql::I64Scalar,
        {
            Value::Int8(self.0.into_val())
        }

        fn visit_u32(self) -> Self::Output
        where
            T: gql::U32Scalar,
        {
            Value::UInt4(self.0.into_val())
        }

        fn visit_u64(self) -> Self::Output
        where
            T: gql::U64Scalar,
        {
            Value::UInt8(self.0.into_val())
        }

        fn visit_string(self) -> Self::Output
        where
            T: gql::StringScalar,
        {
            Value::Text(self.0.into_val())
        }
    }

    T::visit(Visitor(val))
}

/// Convert a [`Value`] to a [`Scalar`].
fn value_to_scalar<T: gql::Scalar>(val: Value) -> Result<T, Error> {
    use gql::Scalar;

    struct Visitor(Value);

    /// We parse a scalar from the row differently depending on the desired type.
    impl<T: Scalar> gql::ScalarVisitor<T> for Visitor {
        type Output = Result<T, Error>;

        fn visit_i32(self) -> Self::Output
        where
            T: gql::I32Scalar,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "i32",
                    got: ty,
                }
            })?))
        }

        fn visit_i64(self) -> Self::Output
        where
            T: Is<Type = i64>,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "i64",
                    got: ty,
                }
            })?))
        }
        fn visit_u32(self) -> Self::Output
        where
            T: Is<Type = u32>,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "u32",
                    got: ty,
                }
            })?))
        }
        fn visit_u64(self) -> Self::Output
        where
            T: Is<Type = u64>,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "u64",
                    got: ty,
                }
            })?))
        }
        fn visit_string(self) -> Self::Output
        where
            T: Is<Type = String>,
        {
            let ty = self.0.ty();
            Ok(T::from_val(self.0.try_into().map_err(|_| {
                Error::TypeMismatch {
                    expected: "string",
                    got: ty,
                }
            })?))
        }
    }

    T::visit(Visitor(val))
}
