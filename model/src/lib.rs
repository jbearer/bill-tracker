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

use derive_more::{Deref, DerefMut, From, Into};
use generic_array::{
    typenum::{UInt, UTerm, Unsigned, B0, B1},
    ArrayLength, GenericArray,
};
use std::fmt::{self, Debug, Formatter};

pub use generic_array::typenum;

pub mod graphql;
pub mod sql;

/// The [`DataSource`](graphql::backend::DataSource) used as a backend for the GraphQL API.
pub use sql::PostgresDataSource as DataSource;

/// A convenience for working with [`GenericArray`].
///
/// The trait [`ArrayLength`] is parameterized by the type of element in the array, which makes it
/// impossible to take as a generic parameter a length which can be used with any type of
/// [`GenericArray`] -- you can't write `N: for<T> ArrayLength<T>`. This is a result of the generic
/// array library predating GATs.
///
/// With GATs, it is perfectly possible to reframe the [`ArrayLength`] trait without parameterizing
/// on the type of array elements, as this trait demonstrates. Now it is possible to write
/// `N: Length` and use like `Array<usize, N>` and `Array<String, N>`.
pub trait Length: Unsigned {
    /// The length of an array of `T`.
    type Of<T>: ArrayLength<T>;
}

impl Length for UTerm {
    type Of<T> = Self;
}

impl<N: Length> Length for UInt<N, B0> {
    type Of<T> = UInt<N::Of<T>, B0>;
}

impl<N: Length> Length for UInt<N, B1> {
    type Of<T> = UInt<N::Of<T>, B1>;
}

/// An array of type `T` with constant length `N`.
#[derive(Clone, Default, Deref, DerefMut, From, Into, PartialEq, Eq)]
pub struct Array<T, N: Length>(GenericArray<T, N::Of<T>>);

impl<T, N: Length> IntoIterator for Array<T, N> {
    type IntoIter = <GenericArray<T, N::Of<T>> as IntoIterator>::IntoIter;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T, N: Length> IntoIterator for &'a Array<T, N> {
    type IntoIter = <&'a GenericArray<T, N::Of<T>> as IntoIterator>::IntoIter;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a, T, N: Length> IntoIterator for &'a mut Array<T, N> {
    type IntoIter = <&'a mut GenericArray<T, N::Of<T>> as IntoIterator>::IntoIter;
    type Item = &'a mut T;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

impl<T, N, B, const M: usize> From<[T; M]> for Array<T, UInt<N, B>>
where
    UInt<N, B>: Length,
    GenericArray<T, <UInt<N, B> as Length>::Of<T>>: From<[T; M]>,
{
    fn from(arr: [T; M]) -> Self {
        Self(GenericArray::from(arr))
    }
}

impl<T> From<[T; 0]> for Array<T, UTerm> {
    fn from(arr: [T; 0]) -> Self {
        Self::from_exact_iter(arr).unwrap()
    }
}

impl<T, N: Length> Array<T, N> {
    /// Creates a new [`Array`] instance from an iterator with a specific size.
    ///
    /// Returns [`None`] if the size is not equal to the number of elements in the [`Array`].
    pub fn from_exact_iter<I>(iter: I) -> Option<Self>
    where
        I: IntoIterator<Item = T>,
    {
        GenericArray::from_exact_iter(iter).map(Self)
    }

    /// Maps an [`Array`] to another [`Array`] with the same length.
    pub fn map<U, F>(self, f: F) -> Array<U, N>
    where
        F: FnMut(T) -> U,
    {
        Array::from_exact_iter(self.into_iter().map(f)).unwrap()
    }

    /// Permutes the contents of `self` by `permutation`.
    ///
    /// This method will reorder the elements of `self` in place using `permutation`, which maps
    /// indices in `self` to their new positions in the permuted version of self.
    ///
    /// # Panics
    ///
    /// Panics if `permutation` is not a permutation of the integers `0..N`.
    pub fn permute(&mut self, permutation: &Array<usize, N>) {
        // The indices in `permutation` we have visited.
        let mut visited = Array::<bool, N>::default();

        // The lowest index in the next cycle within `permutation`.
        let mut next = 0;

        // Permute cycles in `permutation` until there are none left.
        while next < N::USIZE {
            // Permute the first cycle we haven't permuted yet. We can permute a cycle in place by
            // always swapping the next element in the cycle into the slot vacated by the previous.
            let start = next;
            let mut i = start;
            loop {
                visited[i] = true;

                let j = permutation[i];
                assert!(j < N::USIZE, "not a permutation");

                if j == start {
                    // We finished the cycle.
                    break;
                }

                self.swap(i, j);
                i = j;
            }

            // Find the start of the next cycle.
            while next < N::USIZE && visited[next] {
                next += 1;
            }
        }
    }
}

impl<T: Debug, N: Length> Debug for Array<T, N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Create an [`Array`] from a list of items.
///
/// # Examples
///
/// ```
/// # use model::{array, typenum::U3, Array};
/// let arr: Array<usize, U3> = array![usize; 3, 2, 1];
/// assert_eq!(format!("{arr:?}"), "[3, 2, 1]");
/// ```
#[macro_export]
macro_rules! array {
    [$t:ty; $($v:expr),* $(,)?] => {
        $crate::Array::from(generic_array::arr![$t; $($v),*])
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::{prelude::*, test_runner::Config};
    use typenum::{U0, U1, U10};

    #[test]
    fn test_array_permute_empty() {
        let mut arr: Array<usize, U0> = array![usize;];
        arr.permute(&array![usize;]);
        assert_eq!(arr, array![usize;]);
    }

    #[test]
    fn test_array_permute_one() {
        let mut arr: Array<usize, U1> = array![usize; 0];
        arr.permute(&array![usize; 0]);
        assert_eq!(arr, array![usize; 0]);
    }

    fn permutation() -> impl Strategy<Value = Array<usize, U10>> {
        any::<Vec<(usize, usize)>>().prop_map(|swaps| {
            let mut perm = array![usize; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
            for (i, j) in swaps {
                perm.swap(i % 10, j % 10);
            }
            perm
        })
    }

    proptest! {
        #![proptest_config(Config {
            timeout: 100,
            ..Default::default()
        })]

        #[test]
        fn test_array_permute_nonempty(perm in permutation()) {
            let mut array = array![usize; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
            array.permute(&perm);
            prop_assert_eq!(array, perm);
        }
    }
}
