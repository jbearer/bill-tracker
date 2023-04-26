//! Mock instantiation of the abstract [`db`](super) interface for PostgreSQL.
//!
//! This instantiation is built on a simple in-memory database. It is useful for testing in
//! isolation from an actual database.
#![cfg(any(test, feature = "mocks"))]

use super::{Clause, ConstraintKind, SchemaColumn, SelectColumn, Value};
use crate::{Array, Length};
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
    schema: Vec<SchemaColumn<'static>>,
    rows: Vec<Row>,
}

impl Table {
    fn new<N: Length>(schema: Array<SchemaColumn<'static>, N>) -> Self {
        Self {
            schema: schema.to_vec(),
            rows: vec![],
        }
    }

    fn append<N: Length>(&mut self, rows: impl IntoIterator<Item = Array<Value, N>>) {
        assert_eq!(N::USIZE, self.schema.len());
        for row in rows {
            self.rows.push(Row::new(
                row.into_iter()
                    .zip(&self.schema)
                    .map(|(val, col)| (col.name().to_string(), val)),
            ));
        }
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
        self.create_table_with_rows(table, columns, []).await
    }

    /// Create a table with the given column names and row values.
    pub async fn create_table_with_rows<N: Length>(
        &self,
        table: impl Into<String>,
        columns: Array<SchemaColumn<'static>, N>,
        rows: impl IntoIterator<Item = Array<Value, N>>,
    ) -> Result<(), Error> {
        let mut db = self.0.write().await;
        if let Entry::Vacant(e) = db.tables.entry(table.into()) {
            let table = e.insert(Table::new(columns));
            table.append(rows);
        }
        Ok(())
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
            .map(|(name, table)| (name.clone(), table.schema.clone()))
            .collect()
    }
}

impl super::Connection for Connection {
    type Error = Error;
    type CreateTable<'a, N: Length> = CreateTable<'a, N>;
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
        if table.schema.len() != N::USIZE {
            return Err(Error::from(format!(
                "incorrect width for table {} (found {}, expected {})",
                self.table,
                table.schema.len(),
                N::USIZE
            )));
        }

        // A permutation of column indices mapping positions in the input rows to the positions of
        // the corresponding rows in the table schema.
        let mut column_permutation = Array::<usize, N>::default();
        for (i, name) in self.columns.into_iter().enumerate() {
            let col = table
                .schema
                .iter()
                .position(|col| col.name() == name)
                .ok_or_else(|| Error::from(format!("table {} has no column {name}", self.table)))?;
            column_permutation[i] = col;
        }

        for row in &mut self.rows {
            row.permute(&column_permutation);
        }

        table.append(self.rows);
        Ok(())
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
