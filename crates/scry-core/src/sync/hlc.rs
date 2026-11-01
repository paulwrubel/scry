use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A hybrid logical clock timestamp, as it travels over the wire.
///
/// This is deliberately only the value shape: ordering rules, send/receive
/// merging, skew handling, and node-id persistence are tracked by issue #100
/// and will be added there.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Hlc {
    pub wall_ms: u64,
    pub counter: u32,
    pub node_id: Uuid,
}
