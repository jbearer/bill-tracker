//! Instantiation of the data model for a relational database (specifically PostgreSQL).

pub mod data_source;
pub mod db;
mod ops;

pub use data_source::*;
