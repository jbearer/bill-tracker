//! Mock instantiation of the abstract [`db`](super) interface for PostgreSQL.
//!
//! This instantiation is built on a simple in-memory database. It is useful for testing in
//! isolation from an actual database.
#![cfg(any(test, feature = "mocks"))]

use super::{Clause, SelectColumn, Value};
use async_std::sync::{Arc, RwLock};
use derive_more::From;
use futures::{
    stream::{self, BoxStream},
    StreamExt, TryFutureExt,
};
use snafu::Snafu;
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
    schema: Vec<String>,
    rows: Vec<Row>,
}

impl Table {
    fn new<const N: usize>(schema: [impl Into<String>; N]) -> Self {
        Self {
            schema: schema.into_iter().map(|col| col.into()).collect(),
            rows: vec![],
        }
    }

    fn append<const N: usize>(&mut self, rows: impl IntoIterator<Item = [Value; N]>) {
        assert_eq!(N, self.schema.len());
        for row in rows {
            self.rows.push(Row::new(
                row.into_iter()
                    .zip(&self.schema)
                    .map(|(val, col)| (col.clone(), val)),
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
    pub async fn create_table<const N: usize>(
        &self,
        table: impl Into<String>,
        columns: [impl Into<String>; N],
    ) -> Result<(), Error> {
        self.create_table_with_rows(table, columns, []).await
    }

    /// Create a table with the given column names and row values.
    pub async fn create_table_with_rows<const N: usize>(
        &self,
        table: impl Into<String>,
        columns: [impl Into<String>; N],
        rows: impl IntoIterator<Item = [Value; N]>,
    ) -> Result<(), Error> {
        let mut db = self.0.write().await;
        match db.tables.entry(table.into()) {
            Entry::Occupied(e) => Err(Error::from(format!("table {} already exists", e.key()))),
            Entry::Vacant(e) => {
                let table = e.insert(Table::new(columns));
                table.append(rows);
                Ok(())
            }
        }
    }
}

impl super::Connection for Connection {
    type Error = Error;
    type Query<'a> = Query<'a>;

    fn select<'a>(&'a self, _select: &'a [SelectColumn<'a>], table: &'a str) -> Self::Query<'a> {
        Query {
            db: &self.0,
            table,
            clauses: vec![],
        }
    }
}

/// A query against an in-memory database.
pub struct Query<'a> {
    db: &'a RwLock<Db>,
    table: &'a str,
    clauses: Vec<Clause>,
}

impl<'a> super::Query for Query<'a> {
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
                .get(self.table)
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
