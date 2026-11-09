//! Storage root: re-exports the core `Store` contract and the Postgres backend.
pub use scry_core::store::*;

mod postgres;
pub use postgres::*;
