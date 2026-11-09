//! Refresh-token persistence shared by the auth handlers and the store backends.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use scry_core::error::StorageError;
use uuid::Uuid;

/// A refresh token about to be persisted. `token_hash` is the SHA-256 digest of
/// the raw value the client holds.
pub(crate) struct NewRefreshToken {
    pub token_hash: Vec<u8>,
    pub subject: String,
    pub origin_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Outcome of attempting to rotate a refresh token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RefreshRotation {
    /// The old token was valid; the caller may use `subject` for a new access token.
    Rotated { subject: String },
    /// No token matches the presented hash.
    Missing,
    /// The token exists but has passed its expiry.
    Expired,
    /// The token was revoked within the retry grace period.
    Revoked,
    /// The token was revoked long enough ago that presenting it signals reuse.
    Reused,
}

/// How long after revocation a token may still be retried (for example, after a
/// dropped response) before presenting it counts as reuse.
pub(crate) fn reuse_grace() -> chrono::TimeDelta {
    chrono::TimeDelta::seconds(10)
}

#[async_trait]
pub(crate) trait AuthStore: Send + Sync {
    async fn create_refresh_token(&self, token: NewRefreshToken) -> Result<(), StorageError>;

    async fn rotate_refresh_token(
        &self,
        old_hash: &[u8],
        new_hash: &[u8],
        now: DateTime<Utc>,
        new_expires_at: DateTime<Utc>,
    ) -> Result<RefreshRotation, StorageError>;

    async fn revoke_refresh_token(&self, token_hash: &[u8]) -> Result<(), StorageError>;
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// One row in the in-memory refresh-token table.
    #[derive(Debug, Clone)]
    pub(crate) struct MemoryRefreshToken {
        pub subject: String,
        pub origin_id: Uuid,
        pub issued_at: DateTime<Utc>,
        pub expires_at: DateTime<Utc>,
        pub revoked_at: Option<DateTime<Utc>>,
    }

    /// Auth store backed by a `HashMap`, for handler and extractor tests.
    #[derive(Default)]
    pub(crate) struct MemoryAuthStore {
        tokens: Mutex<HashMap<Vec<u8>, MemoryRefreshToken>>,
    }

    impl MemoryAuthStore {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        /// Insert a token with an explicit state, so tests can build expired or
        /// revoked rows without sleeping.
        pub(crate) fn insert(
            &self,
            token_hash: Vec<u8>,
            subject: String,
            origin_id: Uuid,
            issued_at: DateTime<Utc>,
            expires_at: DateTime<Utc>,
            revoked_at: Option<DateTime<Utc>>,
        ) {
            self.tokens.lock().unwrap().insert(
                token_hash,
                MemoryRefreshToken {
                    subject,
                    origin_id,
                    issued_at,
                    expires_at,
                    revoked_at,
                },
            );
        }

        pub(crate) fn get(&self, token_hash: &[u8]) -> Option<MemoryRefreshToken> {
            self.tokens.lock().unwrap().get(token_hash).cloned()
        }
    }

    #[async_trait]
    impl AuthStore for MemoryAuthStore {
        async fn create_refresh_token(&self, token: NewRefreshToken) -> Result<(), StorageError> {
            self.tokens.lock().unwrap().insert(
                token.token_hash,
                MemoryRefreshToken {
                    subject: token.subject,
                    origin_id: token.origin_id,
                    issued_at: token.issued_at,
                    expires_at: token.expires_at,
                    revoked_at: None,
                },
            );
            Ok(())
        }

        async fn rotate_refresh_token(
            &self,
            old_hash: &[u8],
            new_hash: &[u8],
            now: DateTime<Utc>,
            new_expires_at: DateTime<Utc>,
        ) -> Result<RefreshRotation, StorageError> {
            let mut tokens = self.tokens.lock().unwrap();

            let Some(token) = tokens.get(old_hash).cloned() else {
                return Ok(RefreshRotation::Missing);
            };

            if let Some(revoked_at) = token.revoked_at {
                if now.signed_duration_since(revoked_at) <= reuse_grace() {
                    return Ok(RefreshRotation::Revoked);
                }

                let origin_id = token.origin_id;
                for row in tokens.values_mut() {
                    if row.origin_id == origin_id {
                        row.revoked_at = Some(now);
                    }
                }

                return Ok(RefreshRotation::Reused);
            }

            if token.expires_at <= now {
                return Ok(RefreshRotation::Expired);
            }

            if let Some(old) = tokens.get_mut(old_hash) {
                old.revoked_at = Some(now);
            }
            tokens.insert(
                new_hash.to_vec(),
                MemoryRefreshToken {
                    subject: token.subject.clone(),
                    origin_id: token.origin_id,
                    issued_at: now,
                    expires_at: new_expires_at,
                    revoked_at: None,
                },
            );

            Ok(RefreshRotation::Rotated {
                subject: token.subject,
            })
        }

        async fn revoke_refresh_token(&self, token_hash: &[u8]) -> Result<(), StorageError> {
            if let Some(token) = self.tokens.lock().unwrap().get_mut(token_hash) {
                token.revoked_at = Some(Utc::now());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::MemoryAuthStore;
    use super::*;
    use chrono::TimeDelta;

    fn hash(value: &str) -> Vec<u8> {
        value.as_bytes().to_vec()
    }

    fn expiry(now: DateTime<Utc>) -> DateTime<Utc> {
        now + TimeDelta::days(30)
    }

    #[tokio::test]
    async fn valid_token_rotates_and_revokes_the_old_one() {
        let store = MemoryAuthStore::new();
        let now = Utc::now();
        let origin = Uuid::new_v4();
        store.insert(
            hash("old"),
            "local".to_string(),
            origin,
            now,
            expiry(now),
            None,
        );

        let result = store
            .rotate_refresh_token(&hash("old"), &hash("new"), now, expiry(now))
            .await
            .unwrap();

        assert_eq!(
            result,
            RefreshRotation::Rotated {
                subject: "local".to_string()
            }
        );
        assert!(store.get(&hash("old")).unwrap().revoked_at.is_some());
        let new = store.get(&hash("new")).unwrap();
        assert_eq!(new.origin_id, origin);
        assert_eq!(new.subject, "local");
        assert_eq!(new.issued_at, now);
        assert_eq!(new.expires_at, expiry(now));
        assert_eq!(new.revoked_at, None);
    }

    #[tokio::test]
    async fn missing_token_is_missing() {
        let store = MemoryAuthStore::new();
        let now = Utc::now();

        let result = store
            .rotate_refresh_token(&hash("absent"), &hash("new"), now, expiry(now))
            .await
            .unwrap();

        assert_eq!(result, RefreshRotation::Missing);
    }

    #[tokio::test]
    async fn expired_token_is_expired() {
        let store = MemoryAuthStore::new();
        let now = Utc::now();
        store.insert(
            hash("old"),
            "local".to_string(),
            Uuid::new_v4(),
            now - TimeDelta::days(31),
            now - TimeDelta::days(1),
            None,
        );

        let result = store
            .rotate_refresh_token(&hash("old"), &hash("new"), now, expiry(now))
            .await
            .unwrap();

        assert_eq!(result, RefreshRotation::Expired);
        assert!(store.get(&hash("old")).unwrap().revoked_at.is_none());
    }

    #[tokio::test]
    async fn recently_revoked_token_is_revoked_without_lineage_revocation() {
        let store = MemoryAuthStore::new();
        let now = Utc::now();
        let origin = Uuid::new_v4();
        store.insert(
            hash("old"),
            "local".to_string(),
            origin,
            now,
            expiry(now),
            Some(now - TimeDelta::seconds(1)),
        );
        store.insert(
            hash("current"),
            "local".to_string(),
            origin,
            now,
            expiry(now),
            None,
        );

        let result = store
            .rotate_refresh_token(&hash("old"), &hash("new"), now, expiry(now))
            .await
            .unwrap();

        assert_eq!(result, RefreshRotation::Revoked);
        assert!(store.get(&hash("current")).unwrap().revoked_at.is_none());
    }

    #[tokio::test]
    async fn older_revoked_token_reuses_and_revokes_the_lineage() {
        let store = MemoryAuthStore::new();
        let now = Utc::now();
        let origin = Uuid::new_v4();
        store.insert(
            hash("old"),
            "local".to_string(),
            origin,
            now,
            expiry(now),
            Some(now - TimeDelta::seconds(11)),
        );
        store.insert(
            hash("current"),
            "local".to_string(),
            origin,
            now,
            expiry(now),
            None,
        );

        let result = store
            .rotate_refresh_token(&hash("old"), &hash("new"), now, expiry(now))
            .await
            .unwrap();

        assert_eq!(result, RefreshRotation::Reused);
        assert!(store.get(&hash("current")).unwrap().revoked_at.is_some());
    }

    #[tokio::test]
    async fn revoke_sets_revoked_at() {
        let store = MemoryAuthStore::new();
        let now = Utc::now();
        store.insert(
            hash("old"),
            "local".to_string(),
            Uuid::new_v4(),
            now,
            expiry(now),
            None,
        );

        store.revoke_refresh_token(&hash("old")).await.unwrap();

        assert!(store.get(&hash("old")).unwrap().revoked_at.is_some());
    }
}
