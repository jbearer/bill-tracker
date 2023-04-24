//! Abstract interface to a SQL database.

use crate::{Array, Length};
use async_trait::async_trait;
use derive_more::{Display, From, TryInto};
use futures::{Stream, StreamExt, TryStreamExt};
use std::borrow::Cow;
use std::fmt::Display;

pub mod mock;
pub mod postgres;

/// Errors returned by the database.
pub trait Error: Sized + Send + std::error::Error {
    /// Wrap a custom message into this error type.
    fn custom(msg: impl Display) -> Self;

    /// An error indicating that a query returned more than the `expected` number of rows.
    fn too_many_rows(expected: usize) -> Self {
        Self::custom(format!(
            "query result has more rows than the expected {expected}"
        ))
    }

    /// An error indicating that a query which was expected to return some rows did not.
    fn empty_rows() -> Self {
        Self::custom("query result is empty")
    }
}

/// A column in a list of columns selected from a query.
#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SelectColumn<'a> {
    /// A named column
    #[display(fmt = "{}", _0)]
    Col(Cow<'a, str>),
    /// Select all columns.
    #[display(fmt = "*")]
    All,
}

/// A column in a schema.
///
/// This describes the structure and format of each entry in the column, along with column-level
/// metadata like the name and constraints.
#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(fmt = "{name} {ty}")]
pub struct SchemaColumn<'a> {
    name: Cow<'a, str>,
    ty: Type,
}

impl<'a> SchemaColumn<'a> {
    /// Create a column given a name and type.
    pub fn new(name: impl Into<Cow<'a, str>>, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }

    /// The name of this column
    pub fn name(&self) -> Cow<'a, str> {
        self.name.clone()
    }

    /// The type of this column
    pub fn ty(&self) -> Type {
        self.ty
    }

    /// Remove the lifetime requirement from `self` by cloning and taking ownership of borrowed
    /// data.
    pub fn into_static(self) -> SchemaColumn<'static> {
        SchemaColumn {
            name: Cow::Owned(self.name.into_owned()),
            ty: self.ty,
        }
    }
}

/// A connection to the database.
#[async_trait]
pub trait Connection {
    /// Errors returned from queries.
    type Error: Error;

    /// A `SELECT` query which can be executed against the database.
    type Select<'a>: Select<Error = Self::Error>
    where
        Self: 'a;

    /// An `INSERT` statement which can be executed against the database.
    type Insert<'a, N: Length>: Insert<N, Error = Self::Error>
    where
        Self: 'a;

    /// Execute a `CREATE TABLE` statement.
    ///
    /// This will execute a statement of the form `CREATE TABLE IF NOT EXISTS table (columns)`.
    async fn create_table<N: Length>(
        &self,
        table: impl Into<Cow<'_, str>> + Send,
        columns: Array<SchemaColumn<'_>, N>,
    ) -> Result<(), Self::Error>;

    /// Start a `SELECT` query.
    ///
    /// `columns` indicates the columns to include in the query results. The resulting [`Select`]
    /// represents a statement of the form `SELECT columns FROM table`. The query can be refined,
    /// for example by adding a `WHERE` clause, using the approriate methods on the [`Select`] object
    /// before running it.
    fn select<'a>(
        &'a self,
        columns: &'a [SelectColumn<'a>],
        table: impl Into<Cow<'a, str>> + Send,
    ) -> Self::Select<'a>;

    /// Start an `INSERT` query.
    ///
    /// `table` indicates the table to insert into and `columns` the names of the columns in that
    /// table into which values should be inserted.
    fn insert<'a, C, N>(
        &'a self,
        table: impl Into<Cow<'a, str>> + Send,
        columns: Array<C, N>,
    ) -> Self::Insert<'a, N>
    where
        C: Into<String>,
        N: Length;
}

/// A SQL primitive data type.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    #[display(fmt = "text")]
    Text,
    #[display(fmt = "int4")]
    Int4,
    #[display(fmt = "int8")]
    Int8,
    #[display(fmt = "uint4")]
    UInt4,
    #[display(fmt = "uint8")]
    UInt8,
}

/// A primitive value supported by a SQL database.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From, TryInto)]
pub enum Value {
    /// A text string.
    Text(String),
    /// A 4-byte signed integer.
    Int4(i32),
    /// An 8-byte signed integer.
    Int8(i64),
    /// A 4-byte unsigned integer.
    UInt4(u32),
    /// An 8-byte unsigned integer.
    UInt8(u64),
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Self::Text(_) => Type::Text,
            Self::Int4(_) => Type::Int4,
            Self::Int8(_) => Type::Int8,
            Self::UInt4(_) => Type::UInt4,
            Self::UInt8(_) => Type::UInt8,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::Text(s.into())
    }
}

/// A clause modifying a SQL statement.
pub enum Clause {
    /// A `WHERE` clause.
    Where {
        /// The column to filter.
        column: String,
        /// The operation used to filter values of `column`.
        op: String,
        /// Parameter to `op`.
        param: Value,
    },
}

/// A `SELECT` query which can be executed against the database.
pub trait Select: Send {
    /// Errors returned by this query.
    type Error: Error;
    /// Rows returned by this query.
    type Row: Row<Error = Self::Error>;
    /// An asynchronous stream of rows.
    type Stream: Stream<Item = Result<Self::Row, Self::Error>> + Unpin + Send;

    /// Add a clause to the query.
    fn clause(self, clause: Clause) -> Self;

    /// Run the query and get a stream of results.
    fn stream(self) -> Self::Stream;
}

/// An extension trait for [`Select`] that provides some higher-level functions.
#[async_trait]
pub trait SelectExt: Select {
    /// Add a `WHERE` clause to the query.
    fn filter(self, column: impl Into<String>, op: impl Into<String>, param: Value) -> Self;

    /// Run a query which is expected to return a single row.
    ///
    /// # Errors
    ///
    /// This method will fail if the query does not return exactly one row.
    async fn one(self) -> Result<Self::Row, Self::Error>;

    /// Run a query and collect the results.
    async fn many(self) -> Result<Vec<Self::Row>, Self::Error>;

    /// Run a query which is expected to return either 0 or 1 rows.
    ///
    /// # Errors
    ///
    /// This method will fail if the query does not return exactly 0 or 1 rows.
    async fn opt(self) -> Result<Option<Self::Row>, Self::Error>;
}

#[async_trait]
impl<T: Select> SelectExt for T {
    fn filter(self, column: impl Into<String>, op: impl Into<String>, param: Value) -> Self {
        self.clause(Clause::Where {
            column: column.into(),
            op: op.into(),
            param,
        })
    }

    async fn opt(self) -> Result<Option<Self::Row>, Self::Error> {
        let mut rows = self.stream();
        let Some(row) = rows.next().await else { return Ok(None); };
        if rows.next().await.is_some() {
            return Err(Self::Error::too_many_rows(1));
        }
        row.map(Some)
    }

    async fn one(self) -> Result<Self::Row, Self::Error> {
        self.opt().await?.ok_or_else(Self::Error::empty_rows)
    }

    async fn many(self) -> Result<Vec<Self::Row>, Self::Error> {
        self.stream().try_collect().await
    }
}

/// An `INSERT` statement which can be executed against the database.
///
/// The parameter `N` indicates the number of columns in each row to be inserted.
#[async_trait]
pub trait Insert<N: Length>: Send {
    /// Errors returned by this statement.
    type Error: Error;

    /// Add rows to insert.
    fn rows<R>(self, rows: R) -> Self
    where
        R: IntoIterator<Item = Array<Value, N>>;

    /// Do the insertion.
    ///
    /// This will execute a statement of the form `INSERT INTO table (columns) VALUES (rows)`.
    ///
    /// # Errors
    ///
    /// This method will fail if any of the items in `rows` conflict with an existing row in `table`
    /// at a column which is defined as a unique or primary key.
    async fn execute(self) -> Result<(), Self::Error>;
}

/// A row in a database table.
pub trait Row: Sized + Send {
    /// Errors returned by row operations.
    type Error: Error;

    /// Get the value of `column` in this row.
    ///
    /// # Errors
    ///
    /// This method will fail if the specified column does not exist.
    fn column(&self, column: &str) -> Result<Value, Self::Error>;
}
