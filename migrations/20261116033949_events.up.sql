CREATE TABLE events (
    id             UUID PRIMARY KEY,
    project_id     UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title          TEXT NOT NULL,
    description    TEXT,
    event_interval JSONB NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL,
    updated_at     TIMESTAMPTZ NOT NULL
);

CREATE INDEX events_project_id_idx ON events (project_id);
