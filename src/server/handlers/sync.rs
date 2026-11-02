//! Handler for the sync push/pull endpoint.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use scry_core::models::Change;
use scry_core::sync::ack::OperationAcknowledgment;
use scry_core::sync::operations::{
    OPERATION_VERSION, Operation, min_supported_operation_version, operation_version_supported,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::server::ScryState;

/// One push/pull request. `last_seen_seq` is the cursor the client has
/// already applied, and `ops` defaults to empty so a pull-only request can
/// omit it.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Deserialize)]
pub struct SyncRequest {
    pub op_version: u16,
    #[serde(default)]
    pub last_seen_seq: i64,
    #[serde(default)]
    pub ops: Vec<Operation>,
}

/// One push/pull response. `next_seq` is the last `seq` included in
/// `changes`, or the request's `last_seen_seq` when nothing new was found.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
pub struct SyncResponse {
    pub acks: Vec<OperationAcknowledgment>,
    pub changes: Vec<Change>,
    pub next_seq: i64,
}

/// Apply a pushed batch per-op, then return every change after the client's cursor.
pub async fn sync(State(state): State<ScryState>, Json(request): Json<SyncRequest>) -> Response {
    if !operation_version_supported(request.op_version) {
        return AppError::Usage(format!(
            "unsupported operation version {} (this server accepts {}..={})",
            request.op_version,
            min_supported_operation_version(),
            OPERATION_VERSION,
        ))
        .into();
    }

    match state
        .service
        .sync(&request.ops, request.last_seen_seq)
        .await
    {
        Ok(result) => Json(SyncResponse {
            acks: result.acks,
            changes: result.changes,
            next_seq: result.next_seq,
        })
        .into_response(),
        Err(error) => AppError::from(error).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use scry_core::models::{ChangeKind, EntityType};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn request_defaults_cursor_and_ops() {
        let request: SyncRequest = serde_json::from_value(json!({ "op_version": 1 })).unwrap();

        assert_eq!(request.last_seen_seq, 0);
        assert!(request.ops.is_empty());
    }

    #[test]
    fn response_serializes_the_contract() {
        let response = SyncResponse {
            acks: vec![OperationAcknowledgment::applied(Uuid::new_v4())],
            changes: vec![Change {
                seq: 4,
                entity_type: EntityType::Task,
                entity_id: Uuid::new_v4(),
                kind: ChangeKind::Update,
                operation_id: Uuid::new_v4(),
                client_timestamp: None,
                field_values: json!({ "title": "new" }),
                created_at: Utc::now(),
            }],
            next_seq: 4,
        };

        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(value["acks"][0]["status"], json!("applied"));
        assert_eq!(value["changes"][0]["seq"], json!(4));
        assert_eq!(value["changes"][0]["entity_type"], json!("task"));
        assert_eq!(value["changes"][0]["kind"], json!("update"));
        assert_eq!(value["next_seq"], json!(4));
    }

    #[test]
    fn change_round_trips() {
        let change = Change {
            seq: 7,
            entity_type: EntityType::Note,
            entity_id: Uuid::new_v4(),
            kind: ChangeKind::Create,
            operation_id: Uuid::new_v4(),
            client_timestamp: None,
            field_values: json!({ "contents": "hello" }),
            created_at: Utc::now(),
        };

        let encoded = serde_json::to_string(&change).unwrap();
        let decoded: Change = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, change);
    }
}
