-- Browser refresh tokens. Only the SHA-256 digest is stored; the raw token
-- lives solely in the client's HttpOnly cookie. `origin_id` ties a rotated
-- token's lineage together so replaying an old token can revoke the family.
CREATE TABLE refresh_tokens (
    token_hash BYTEA PRIMARY KEY,
    subject    TEXT NOT NULL,
    origin_id  UUID NOT NULL,
    issued_at  TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NULL
);

CREATE INDEX refresh_tokens_origin_id_idx ON refresh_tokens (origin_id);
