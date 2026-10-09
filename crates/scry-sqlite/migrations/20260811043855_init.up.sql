CREATE TABLE IF NOT EXISTS projects (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL UNIQUE,
    created_at   TEXT NOT NULL
);
INSERT INTO projects (id, name, created_at) VALUES (1, 'default', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

CREATE TABLE IF NOT EXISTS statuses (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id   INTEGER NOT NULL REFERENCES projects(id),
    name         TEXT NOT NULL,
    position     INTEGER NOT NULL DEFAULT 0,
    color        TEXT,
    style        TEXT,
    UNIQUE(project_id, name)
);
INSERT INTO statuses (id, project_id, name, position, style) VALUES (1, 1, 'todo', 0, 'default');
INSERT INTO statuses (id, project_id, name, position, style) VALUES (2, 1, 'done', 1, 'completed');

CREATE TABLE IF NOT EXISTS tasks (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    description  TEXT,
    created_at   TEXT NOT NULL,
    project_id   INTEGER NOT NULL DEFAULT 1 REFERENCES projects(id),
    status_id    INTEGER NOT NULL DEFAULT 1 REFERENCES statuses(id),
    position     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT OR REPLACE INTO config (key, value) VALUES ('active_project', 'default');
