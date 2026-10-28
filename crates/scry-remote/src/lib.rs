//! HTTP client [`Store`](scry_core::store::Store) implementation for scry.
//!
//! [`RemoteStore`] talks to scry's `/api/v1` REST API, letting the CLI and other
//! tooling use a remote server in place of a local database.

mod remote;

pub use remote::RemoteStore;
