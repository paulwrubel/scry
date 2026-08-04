CREATE TABLE tasks_new (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    description  TEXT,
    created_at   TEXT NOT NULL,
    completed_at TEXT,
    project_id   INTEGER NOT NULL DEFAULT 1 REFERENCES projects(id),
    state_id     INTEGER NOT NULL DEFAULT 1 REFERENCES states(id)
);

INSERT INTO tasks_new (id, title, description, created_at, completed_at, project_id, state_id)
    SELECT id, title, description, created_at, completed_at, 1,
           CASE WHEN is_complete = 1 THEN 2 ELSE 1 END
    FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;
