//! Per-operation acknowledgements produced by the server's apply pipeline.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The server's acknowledgement of one operation, keyed by the id the client
/// assigned to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OperationAcknowledgment {
    pub operation_id: Uuid,
    // flattened so an ack is a flat object the client can match on `status`
    #[serde(flatten)]
    pub status: AcknowledgmentStatus,
}

impl OperationAcknowledgment {
    pub fn applied(operation_id: Uuid) -> Self {
        Self {
            operation_id,
            status: AcknowledgmentStatus::Applied,
        }
    }

    pub fn rejected(operation_id: Uuid, reason: RejectionReason) -> Self {
        Self {
            operation_id,
            status: AcknowledgmentStatus::Rejected { reason },
        }
    }
}

/// Whether an operation was applied or why it was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AcknowledgmentStatus {
    Applied,
    Rejected {
        // flattened so the reason's own tag sits beside `status` rather than
        // nesting under another `reason` key
        #[serde(flatten)]
        reason: RejectionReason,
    },
}

/// Why the server refused an operation. `SectionMerged` arrives with the merge
/// step in #108, which is also what produces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum RejectionReason {
    Tombstoned,
    NotFound,
    Conflict,
    UnsupportedVersion,
}

impl RejectionReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tombstoned => "tombstoned",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::UnsupportedVersion => "unsupported_version",
        }
    }
}

/// The result of applying one batch of operations.
#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    pub acks: Vec<OperationAcknowledgment>,
    /// The highest changes-log `seq` appended by this batch, or `None` when no
    /// operation produced a change.
    pub last_seq: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn applied_ack_serializes_without_a_reason() {
        let id = Uuid::new_v4();
        let ack = OperationAcknowledgment::applied(id);

        assert_eq!(
            serde_json::to_value(&ack).unwrap(),
            json!({ "operation_id": id, "status": "applied" })
        );
        assert_eq!(
            serde_json::to_string(&ack).unwrap(),
            format!(r#"{{"operation_id":"{id}","status":"applied"}}"#)
        );
    }

    #[test]
    fn rejected_ack_round_trips() {
        let ack = OperationAcknowledgment::rejected(Uuid::new_v4(), RejectionReason::NotFound);
        let encoded = serde_json::to_string(&ack).unwrap();
        let decoded: OperationAcknowledgment = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, ack);
    }

    #[test]
    fn tombstoned_rejection_serializes_the_reason_beside_the_status() {
        let id = Uuid::new_v4();
        let ack = OperationAcknowledgment::rejected(id, RejectionReason::Tombstoned);

        let expected =
            format!(r#"{{"operation_id":"{id}","status":"rejected","reason":"tombstoned"}}"#);
        assert_eq!(serde_json::to_string(&ack).unwrap(), expected);
        assert_eq!(
            serde_json::to_value(&ack).unwrap(),
            json!({ "operation_id": id, "status": "rejected", "reason": "tombstoned" })
        );
    }

    #[test]
    fn reject_reason_strings_are_snake_case() {
        assert_eq!(RejectionReason::Tombstoned.as_str(), "tombstoned");
        assert_eq!(RejectionReason::NotFound.as_str(), "not_found");
        assert_eq!(RejectionReason::Conflict.as_str(), "conflict");
        assert_eq!(
            RejectionReason::UnsupportedVersion.as_str(),
            "unsupported_version"
        );
    }
}
