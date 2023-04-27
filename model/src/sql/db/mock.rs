//! Mock instantiation of the abstract [`db`](super) interface for PostgreSQL.
//!
//! This instantiation is built on a simple in-memory database. It is useful for testing in
//! isolation from an actual database.
#![cfg(any(test, feature = "mocks"))]

use super::{Clause, ConstraintKind, SchemaColumn, SelectColumn, Type, Value};
use crate::{
    typenum::{Sub1, B1},
    Array, Length,
};
use async_std::sync::{Arc, RwLock};
use async_trait::async_trait;
use derive_more::From;
use futures::{
    stream::{self, BoxStream},
    StreamExt, TryFutureExt,
};
use snafu::Snafu;
use std::borrow::Cow;
use std::collections::hash_map::{Entry, HashMap};
use std::fmt::Display;
use std::ops::Sub;

/// Errors returned by the in-memory database.
#[derive(Debug, Snafu, From)]
#[snafu(display("mock DB error: {}", message))]
pub struct Error {
    message: String,
}

impl super::Error for Error {
    fn custom(msg: impl Display) -> Self {
        Self {
            message: msg.to_string(),
        }
    }
}

/// The in-memory database.
#[derive(Debug, Default)]
struct Db {
    tables: HashMap<String, Table>,
}

/// An in-memory table.
#[derive(Debug)]
struct Table {
    name: String,
    serial_cols: Vec<SchemaColumn<'static>>,
    explicit_cols: Vec<SchemaColumn<'static>>,
    rows: Vec<Row>,
}

impl Table {
    fn new<N: Length>(name: String, schema: Array<SchemaColumn<'static>, N>) -> Self {
        // Separate the auto-incrementing columns from the columns that require explicit values.
        let (serial_cols, explicit_cols) =
            schema.into_iter().partition(|col| col.ty() == Type::Serial);
        Self {
            name,
            serial_cols,
            explicit_cols,
            rows: vec![],
        }
    }

    fn append<N: Length>(
        &mut self,
        rows: impl IntoIterator<Item = Array<Value, N>>,
    ) -> Result<(), Error> {
        // We require a value for all columns except the serial columns (which are auto-incremented).
        if N::USIZE != self.explicit_cols.len() {
            return Err(Error::from(format!(
                "incorrect width for table {} (found {}, expected {})",
                self.name,
                self.explicit_cols.len(),
                N::USIZE
            )));
        }

        for row in rows {
            // Auto-increment the serial columns.
            let auto_values = self
                .serial_cols
                .iter()
                .map(|col| (col.name().to_string(), (self.rows.len() as i32 + 1).into()));

            // Take the rest of the values from the input.
            let values = row
                .into_iter()
                .zip(&self.explicit_cols)
                .map(|(val, col)| (col.name().to_string(), val));

            self.rows.push(Row::new(auto_values.chain(values)));
        }

        Ok(())
    }

    fn schema(&self) -> Vec<SchemaColumn<'static>> {
        self.serial_cols
            .iter()
            .chain(&self.explicit_cols)
            .cloned()
            .collect()
    }
}

/// A connection to the in-memory database.
#[derive(Clone, Debug)]
pub struct Connection(Arc<RwLock<Db>>);

impl Connection {
    /// Create a new database and connect to it.
    ///
    /// This will create a connection to a fresh, empty database. It will not be connected or
    /// related to any previous connection or database. Once the database is created, this
    /// connection can be [cloned](Clone) in order to create multiple simultaneous connections to
    /// the same database.
    pub fn create() -> Self {
        Self(Default::default())
    }

    /// Create a table with the given column names.
    pub async fn create_table<N: Length>(
        &self,
        table: impl Into<String>,
        columns: Array<SchemaColumn<'static>, N>,
    ) -> Result<(), Error> {
        let mut db = self.0.write().await;
        let table = table.into();
        if let Entry::Vacant(e) = db.tables.entry(table.clone()) {
            e.insert(Table::new(table, columns));
        }
        Ok(())
    }

    /// Create a table with the given column names and row values.
    ///
    /// It is assumed that the schema contains exactly 1 auto-increment ID column, so the values
    /// specified for each row must be 1 less than the size of the schema.
    pub async fn create_table_with_rows<N: Length + Sub<B1>>(
        &self,
        table: impl Into<String>,
        columns: Array<SchemaColumn<'static>, N>,
        rows: impl IntoIterator<Item = Array<Value, Sub1<N>>>,
    ) -> Result<(), Error>
    where
        Sub1<N>: Length,
    {
        let table = table.into();
        self.create_table(&table, columns).await?;

        let mut db = self.0.write().await;
        let table = db.tables.get_mut(&table).unwrap();
        table.append(rows)
    }

    /// The schema of this database.
    ///
    /// The schema maps table names to the schema for each table. Each table schema consists of a
    /// list of column schemas.
    pub async fn schema(&self) -> HashMap<String, Vec<SchemaColumn<'static>>> {
        self.0
            .read()
            .await
            .tables
            .iter()
            .map(|(name, table)| (name.clone(), table.schema()))
            .collect()
    }
}

impl super::Connection for Connection {
    type Error = Error;
    type CreateTable<'a, N: Length> = CreateTable<'a, N>;
    type AlterTable<'a> = AlterTable;
    type Select<'a> = Select<'a>;
    type Insert<'a, N: Length> = Insert<'a, N>;

    fn create_table<'a, N: Length>(
        &'a self,
        table: impl Into<Cow<'a, str>> + Send,
        columns: Array<SchemaColumn<'a>, N>,
    ) -> Self::CreateTable<'a, N> {
        CreateTable {
            db: self,
            table: table.into(),
            columns,
        }
    }

    fn alter_table<'a>(&'a self, _table: impl Into<Cow<'a, str>> + Send) -> Self::AlterTable<'a> {
        AlterTable
    }

    fn select<'a>(
        &'a self,
        _select: &'a [SelectColumn<'a>],
        table: impl Into<Cow<'a, str>> + Send,
    ) -> Self::Select<'a> {
        Select {
            db: &self.0,
            table: table.into(),
            clauses: vec![],
        }
    }

    fn insert<'a, C, N: Length>(
        &'a self,
        table: impl Into<Cow<'a, str>> + Send,
        columns: Array<C, N>,
    ) -> Self::Insert<'a, N>
    where
        C: Into<String>,
    {
        Insert {
            db: &self.0,
            table: table.into(),
            columns: columns.map(|c| c.into()),
            rows: vec![],
        }
    }
}

/// A query against an in-memory database.
pub struct Select<'a> {
    db: &'a RwLock<Db>,
    table: Cow<'a, str>,
    clauses: Vec<Clause>,
}

impl<'a> super::Select for Select<'a> {
    type Error = Error;
    type Row = Row;
    type Stream = BoxStream<'a, Result<Self::Row, Self::Error>>;

    fn clause(mut self, clause: Clause) -> Self {
        self.clauses.push(clause);
        self
    }

    fn stream(self) -> Self::Stream {
        async move {
            let db = self.db.read().await;
            let table = db
                .tables
                .get(&*self.table)
                .ok_or_else(|| Error::from(format!("no such table {}", self.table)))?;
            let rows = table
                .rows
                .clone()
                .into_iter()
                .filter(move |row| self.clauses.iter().all(|clause| row.test(clause)))
                .map(Ok);
            Ok(stream::iter(rows))
        }
        .try_flatten_stream()
        .boxed()
    }
}

/// An insert statement for an in-memory database.
pub struct Insert<'a, N: Length> {
    db: &'a RwLock<Db>,
    table: Cow<'a, str>,
    columns: Array<String, N>,
    rows: Vec<Array<Value, N>>,
}

#[async_trait]
impl<'a, N: Length> super::Insert<N> for Insert<'a, N> {
    type Error = Error;

    fn rows<R>(mut self, rows: R) -> Self
    where
        R: IntoIterator<Item = Array<Value, N>>,
    {
        self.rows.extend(rows);
        self
    }

    async fn execute(mut self) -> Result<(), Error> {
        let mut db = self.db.write().await;
        let table = db
            .tables
            .get_mut(&*self.table)
            .ok_or_else(|| Error::from(format!("no such table {}", self.table)))?;

        // A permutation of column indices mapping positions in the input rows to the positions of
        // the corresponding rows in the table schema.
        let mut column_permutation = Array::<usize, N>::default();
        for (i, name) in self.columns.into_iter().enumerate() {
            let col = table
                .explicit_cols
                .iter()
                .position(|col| col.name() == name)
                .ok_or_else(|| Error::from(format!("table {} has no column {name}", self.table)))?;
            column_permutation[i] = col;
        }

        for row in &mut self.rows {
            row.permute(&column_permutation);
        }

        table.append(self.rows)
    }
}

/// A create table statement for an in-memory database.
pub struct CreateTable<'a, N: Length> {
    db: &'a Connection,
    table: Cow<'a, str>,
    columns: Array<SchemaColumn<'a>, N>,
}

#[async_trait]
impl<'a, N: Length> super::CreateTable for CreateTable<'a, N> {
    type Error = Error;

    fn constraint<I>(self, _kind: ConstraintKind, _columns: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        // The mock database doesn't enforce constraints.
        self
    }

    async fn execute(self) -> Result<(), Self::Error> {
        self.db
            .create_table(self.table, self.columns.map(|col| col.into_static()))
            .await
    }
}

/// An alter table statement for an in-memory database.
pub struct AlterTable;

#[async_trait]
impl super::AlterTable for AlterTable {
    type Error = Error;

    fn add_constraint<I>(self, _kind: ConstraintKind, _columns: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        // The mock database doesn't enforce constraints.
        self
    }

    async fn execute(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// A row in an in-memory table.
#[derive(Clone, Debug, Default)]
pub struct Row {
    columns: HashMap<String, Value>,
}

macro_rules! test_int_val {
    ($l:expr, $op:expr, $r:expr, $($t:ident),+) => {
        match ($l, $r) {
            $(
                (Value::$t(l), Value::$t(r)) => match $op {
                    ">" => l > r,
                    ">=" => l >= r,
                    "<" => l < r,
                    "<=" => l <= r,
                    op => panic!("unsupported int op {op}"),
                }
            ),+
            (l, r) => panic!("type mismatch for op {}: {:?}, {:?}", $op, l, r),
        }
    };
    ($l:expr, $op:expr, $r:expr) => {
        test_int_val!($l, $op, $r, Int4, Int8, UInt4, UInt8)
    };
}

impl Row {
    /// Create a row with the given entries.
    fn new(entries: impl IntoIterator<Item = (String, Value)>) -> Self {
        Self {
            columns: entries.into_iter().collect(),
        }
    }

    /// Test if this row should be included based on the given [`Clause`].
    fn test(&self, clause: &Clause) -> bool {
        match clause {
            Clause::Where { column, op, param } => {
                if let Some(col) = self.columns.get(column) {
                    match op.as_str() {
                        "=" => col == param,
                        "!=" => col != param,
                        int_op => test_int_val!(col, int_op, param),
                    }
                } else {
                    true
                }
            }
        }
    }
}

impl super::Row for Row {
    type Error = Error;

    fn column(&self, column: &str) -> Result<Value, Self::Error> {
        self.columns
            .get(column)
            .cloned()
            .ok_or_else(|| format!("no such column {column}").into())
    }
}
