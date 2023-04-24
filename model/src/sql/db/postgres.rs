//! Instantiation of the abstract [`db`](super) interface for PostgreSQL.
//!
//! This instantiation is built on [`async-postgres`].

use super::{Clause, SchemaColumn, SelectColumn, Value};
use crate::{Array, Length};
use async_std::task::spawn;
use async_trait::async_trait;
use bytes::BytesMut;
use derive_more::From;
use futures::{
    future,
    stream::{self, BoxStream},
    StreamExt, TryFutureExt, TryStreamExt,
};
use itertools::Itertools;
use snafu::Snafu;
use std::borrow::Cow;
use std::fmt::Display;
use tokio_postgres::{
    types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type},
    NoTls,
};

pub use async_postgres::{Config, Row};

/// Errors returned by a PostgreSQL database.
#[derive(Debug, Snafu, From)]
pub enum Error {
    #[from]
    Sql {
        source: async_postgres::Error,
    },
    OutOfRange {
        ty: &'static str,
        value: String,
    },
    UnsupportedType {
        ty: Type,
    },
    Custom {
        message: String,
    },
}

impl super::Error for Error {
    fn custom(msg: impl Display) -> Self {
        Self::Custom {
            message: msg.to_string(),
        }
    }
}

/// A connection to a PostgreSQL databsae.
pub struct Connection(tokio_postgres::Client);

impl Connection {
    /// Establish a new connection with the given [`Config`].
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let (client, conn) = config.connect(NoTls).await?;
        spawn(conn);
        Ok(Self(client))
    }
}

#[async_trait]
impl super::Connection for Connection {
    type Error = Error;
    type Select<'a> = Select<'a>;
    type Insert<'a, N: Length> = Insert<'a, N>;

    async fn create_table<N: Length>(
        &self,
        table: impl Into<Cow<'_, str>> + Send,
        columns: Array<SchemaColumn<'_>, N>,
    ) -> Result<(), Self::Error> {
        let table = table.into();
        let columns = columns
            .into_iter()
            .map(|col| {
                let ty = match col.ty() {
                    super::Type::Int4 => "int4",
                    super::Type::Int8 => "int8",
                    super::Type::UInt4 => "int8",
                    super::Type::UInt8 => "int8",
                    super::Type::Text => "text",
                };
                format!("{} {}", col.name(), ty)
            })
            .join(",");
        self.0
            .execute(
                format!("CREATE TABLE IF NOT EXISTS {table} ({columns})").as_str(),
                &[],
            )
            .await?;
        Ok(())
    }

    fn select<'a>(
        &'a self,
        select: &'a [SelectColumn<'a>],
        table: impl Into<Cow<'a, str>> + Send,
    ) -> Self::Select<'a> {
        Select::new(self, select, table.into())
    }

    fn insert<'a, C, N: Length>(
        &'a self,
        table: impl Into<Cow<'a, str>> + Send,
        columns: Array<C, N>,
    ) -> Self::Insert<'a, N>
    where
        C: Into<String>,
    {
        Insert::new(self, table.into(), columns)
    }
}

/// A query against a PostgreSQL database.
pub struct Select<'a>(Result<SelectInner<'a>, Error>);

struct SelectInner<'a> {
    conn: &'a Connection,
    select: &'a [SelectColumn<'a>],
    table: Cow<'a, str>,
    conditions: Vec<String>,
    params: Vec<Value>,
}

impl<'a> Select<'a> {
    fn new(conn: &'a Connection, select: &'a [SelectColumn<'a>], table: Cow<'a, str>) -> Self {
        Self(Ok(SelectInner {
            conn,
            select,
            table,
            conditions: Default::default(),
            params: Default::default(),
        }))
    }
}

impl<'a> super::Select for Select<'a> {
    type Error = Error;
    type Row = Row;
    type Stream = BoxStream<'a, Result<Self::Row, Self::Error>>;

    fn clause(self, clause: Clause) -> Self {
        let Ok(mut query) = self.0 else { return self; };
        match clause {
            Clause::Where { column, op, param } => {
                query.params.push(param);
                query
                    .conditions
                    .push(format!("{column} {op} ${}", query.params.len()));
            }
        }
        Self(Ok(query))
    }

    fn stream(self) -> Self::Stream {
        let query = match self.0 {
            Ok(query) => query,
            Err(err) => return stream::once(future::ready(Err(err))).boxed(),
        };

        // The async block is necessary to move data owned by `query` into the future, so we can
        // return the future without returning a reference to the local `query`.
        async move {
            // Format the `SELECT` part of the query.
            let columns = query.select.iter().map(|col| col.to_string()).join(", ");
            let table = query.table;

            // Format the `WHERE` clause if there is one.
            let clauses = if query.conditions.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", query.conditions.into_iter().join(" AND "))
            };

            // Construct the SQL statement.
            let statement = format!("SELECT {columns} FROM {table} {clauses}");

            // Borrow parameters.
            let params = query.params.iter().map(|param| {
                let param: &dyn ToSql = param;
                param
            });

            // Run the query.
            query.conn.0.query_raw(statement.as_str(), params).await
        }
        .try_flatten_stream()
        .map_err(Error::from)
        .boxed()
    }
}

/// An `INSERT` statement for a PostgreSQL database.
pub struct Insert<'a, N: Length> {
    conn: &'a Connection,
    table: Cow<'a, str>,
    columns: Array<String, N>,
    num_rows: usize,
    params: Vec<Value>,
}

impl<'a, N: Length> Insert<'a, N> {
    fn new<C: Into<String>>(
        conn: &'a Connection,
        table: Cow<'a, str>,
        columns: Array<C, N>,
    ) -> Self {
        Self {
            conn,
            table,
            columns: columns.map(|c| c.into()),
            num_rows: 0,
            params: vec![],
        }
    }
}

#[async_trait]
impl<'a, N: Length> super::Insert<N> for Insert<'a, N> {
    type Error = Error;

    fn rows<R>(mut self, rows: R) -> Self
    where
        R: IntoIterator<Item = Array<Value, N>>,
    {
        for row in rows {
            self.params.extend(row);
            self.num_rows += 1;
        }
        self
    }

    async fn execute(self) -> Result<(), Error> {
        let columns = self.columns.iter().join(",");
        let rows = (0..self.num_rows)
            .map(|i| {
                let values = (0..N::USIZE)
                    .map(|j| {
                        // In the query itself, just reference a parameter by number. We will pass
                        // the value itself into the query as a parameter to prevent SQL injection.
                        let param_num = i * N::USIZE + j;
                        // Params are 1-indexed.
                        (param_num + 1).to_string()
                    })
                    .join(",");
                format!("({values})")
            })
            .join(",");
        let params = self.params.iter().map(|param| {
            let param: &dyn ToSql = param;
            param
        });
        self.conn
            .0
            .execute_raw(
                format!("INSERT INTO {} ({}) VALUES {}", self.table, columns, rows).as_str(),
                params,
            )
            .await?;
        Ok(())
    }
}

impl super::Row for Row {
    type Error = Error;

    fn column(&self, column: &str) -> Result<Value, Self::Error> {
        Ok(self.try_get(column)?)
    }
}

impl ToSql for Value {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync + 'static>>
    where
        Self: Sized,
    {
        match self {
            Self::Text(x) => x.to_sql(ty, out),
            Self::Int4(x) => x.to_sql(ty, out),
            Self::Int8(x) => x.to_sql(ty, out),
            Self::UInt4(x) => x.to_sql(ty, out),
            Self::UInt8(x) => {
                // [`u64`] doesn't implement [`ToSql`], so we have to cast to a [`u32`] first.
                let x = u32::try_from(*x).map_err(|_| {
                    Box::new(Error::OutOfRange {
                        ty: "u32",
                        value: x.to_string(),
                    })
                })?;
                x.to_sql(ty, out)
            }
        }
    }

    accepts!(INT4, INT8, TEXT);
    to_sql_checked!();
}

impl<'a> FromSql<'a> for Value {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        match ty {
            &Type::INT4 => Ok(Self::Int4(i32::from_sql(ty, raw)?)),
            &Type::INT8 => Ok(Self::Int8(i64::from_sql(ty, raw)?)),
            &Type::TEXT => Ok(Self::Text(String::from_sql(ty, raw)?)),
            ty => Err(Box::new(Error::UnsupportedType { ty: ty.clone() })),
        }
    }

    accepts!(INT4, INT8, TEXT);
}
