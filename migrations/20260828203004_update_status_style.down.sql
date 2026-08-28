-- Add down migration script here
-- Reverse the up migration: drop the NOT NULL constraint and restore the old style values.
PRAGMA defer_foreign_keys = ON;

CREATE TEMP TABLE statuses_backup AS SELECT id, project_id, name, position, color, style FROM statuses;

DROP TABLE statuses;

CREATE TABLE statuses (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id   INTEGER NOT NULL REFERENCES projects(id),
    name         TEXT NOT NULL,
    position     INTEGER NOT NULL DEFAULT 0,
    color        TEXT,
    style        TEXT,
    UNIQUE(project_id, name)
);

INSERT INTO statuses (id, project_id, name, position, color, style)
    SELECT id, project_id, name, position, color,
       CASE WHEN style IN ('checked', 'strikethrough') THEN 'completed' ELSE 'default' END
    FROM statuses_backup;

DROP TABLE statuses_backup;
