
CREATE TABLE IF NOT EXISTS states (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id   INTEGER NOT NULL REFERENCES projects(id),
    name         TEXT NOT NULL,
    position     INTEGER NOT NULL DEFAULT 0,

    UNIQUE(project_id, name)
);
INSERT INTO states (id, project_id, name, position) VALUES (1, 1, 'todo', 0);
INSERT INTO states (id, project_id, name, position) VALUES (2, 1, 'done', 1);

