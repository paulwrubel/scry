CREATE TABLE projects (
    id                UUID PRIMARY KEY,
    name              TEXT NOT NULL UNIQUE,
    description       TEXT,
    task_sorting_mode TEXT NOT NULL DEFAULT 'alphabetical',
    show_priority     BOOLEAN NOT NULL DEFAULT false,
    created_at        TIMESTAMPTZ NOT NULL
);
-- stable UUID of the built-in default project: UUIDv5(UUIDv5(NAMESPACE_DNS, "scry"), "default")
INSERT INTO projects (id, name, created_at) VALUES ('b4bf48f3-2fdb-5b9f-b55f-2f4ffc38bb08', 'default', now());

CREATE TABLE sections (
    id         UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0,
    color      TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sections_project_id_position_idx ON sections (project_id, position);

CREATE TABLE tasks (
    id           UUID PRIMARY KEY,
    project_id   UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    description  TEXT,
    priority     BIGINT NOT NULL DEFAULT 3,
    section_id   UUID REFERENCES sections(id) ON DELETE SET NULL,
    position     INTEGER NOT NULL DEFAULT 0,
    closed_at    TIMESTAMPTZ,
    close_reason TEXT,
    tags         TEXT NOT NULL DEFAULT '',
    created_at   TIMESTAMPTZ NOT NULL
);
CREATE INDEX tasks_project_id_section_id_position_idx ON tasks (project_id, section_id, position);

CREATE TABLE notes (
    id         UUID PRIMARY KEY,
    task_id    UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    contents   TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX notes_task_id_idx ON notes (task_id);
