CREATE TABLE projects (
    id                UUID PRIMARY KEY,
    name              TEXT NOT NULL UNIQUE,
    description       TEXT,
    task_sorting_mode TEXT NOT NULL,
    show_priority     BOOLEAN NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL
);
-- stable UUID of the built-in default project: UUIDv5(UUIDv5(NAMESPACE_DNS, "scry"), "default")
INSERT INTO projects (id, name, description, task_sorting_mode, show_priority, created_at)
VALUES (
    'b4bf48f3-2fdb-5b9f-b55f-2f4ffc38bb08', 
    'the default project - auto-created for you!', 
    NULL, 
    'alphabetical', 
    false, 
    now()
);

CREATE TABLE sections (
    id         UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   TEXT COLLATE "C" NOT NULL,
    color      TEXT,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX sections_project_id_position_idx ON sections (project_id, position);

CREATE TABLE tasks (
    id           UUID PRIMARY KEY,
    project_id   UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    description  TEXT,
    priority     BIGINT NOT NULL,
    section_id   UUID REFERENCES sections(id) ON DELETE SET NULL,
    position     TEXT COLLATE "C" NOT NULL,
    state        TEXT NOT NULL,
    tags         TEXT NOT NULL,
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
