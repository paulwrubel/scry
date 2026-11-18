CREATE TABLE event_overrides (
    event_id       UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    original_start JSONB NOT NULL,
    title          TEXT,
    description    TEXT,
    event_interval JSONB,
    UNIQUE (event_id, original_start)
);
