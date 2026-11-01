-- Dedupe ledger: an operation id is applied at most once, forever.
CREATE TABLE applied_operations (
    operation_id UUID PRIMARY KEY,
    applied_at   TIMESTAMPTZ NOT NULL
);

-- Ordered log of the field values each applied operation produced. Clients pull
-- `seq > cursor` and overwrite their confirmed state with `field_values`.
-- `entity_type` is one of 'project', 'section', 'task', or 'note'.
-- `kind` is 'create', 'update', or 'delete'.
-- Identity only makes `seq` monotonic; it does not order commits across
-- concurrent transactions, so the apply pipeline must serialize appends.
CREATE TABLE changes (
    seq              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    entity_type      TEXT NOT NULL,
    entity_id        UUID NOT NULL,
    kind             TEXT NOT NULL,
    operation_id     UUID NOT NULL,
    client_timestamp TIMESTAMPTZ,
    field_values     JSONB NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL
);

-- Deletes are permanent: a tombstone is never removed, so a client is never
-- told to resync because a delete aged out.
CREATE TABLE entity_tombstones (
    entity_type TEXT NOT NULL,
    entity_id   UUID NOT NULL,
    deleted_at  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (entity_type, entity_id)
);
