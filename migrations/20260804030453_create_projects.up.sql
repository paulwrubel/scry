
CREATE TABLE IF NOT EXISTS projects (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL UNIQUE,
    created_at   TEXT NOT NULL
);

INSERT INTO projects (id, name, created_at) VALUES (1, 'default', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

