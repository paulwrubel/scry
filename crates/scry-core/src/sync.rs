//! The sync wire contract shared by every peer: the operation vocabulary and the
//! optional client timestamp attached to operations.

pub mod ack;
mod double_option;
pub mod operations;
