CREATE TABLE automations (
    id         UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    trigger    JSONB NOT NULL,
    actions    JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX automations_project_id_idx ON automations (project_id);
