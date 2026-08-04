CREATE TABLE tasks_old (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    title           TEXT NOT NULL,
    description     TEXT,
    is_complete     INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    completed_at    TEXT
);

INSERT INTO tasks_old (id, title, description, is_complete, created_at, completed_at)
    SELECT id, title, description,
           CASE WHEN state_id = 2 THEN 1 ELSE 0 END,
           created_at, completed_at
    FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_old RENAME TO tasks;
