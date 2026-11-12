//! The sync wire contract shared by every peer: the operation vocabulary and the
//! optional client timestamp attached to operations.

pub mod ack;
pub mod double_option;
pub mod operations;
pub mod replay;
