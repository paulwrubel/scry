-- Add up migration script here
-- Normalize status style values and make the style column NOT NULL.
-- SQLite cannot add a NOT NULL constraint via ALTER TABLE, so the table is rebuilt.
-- sqlx runs migrations inside a transaction, which makes PRAGMA foreign_keys=OFF a no-op;
-- defer_foreign_keys instead defers the FK check on DROP TABLE until COMMIT, by which point
-- the rebuilt table (same name, same ids) satisfies the constraint.
PRAGMA defer_foreign_keys = ON;

CREATE TEMP TABLE statuses_backup AS SELECT id, project_id, name, position, color, style FROM statuses;

DROP TABLE statuses;

CREATE TABLE statuses (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id   INTEGER NOT NULL REFERENCES projects(id),
    name         TEXT NOT NULL,
    position     INTEGER NOT NULL DEFAULT 0,
    color        TEXT,
    style        TEXT NOT NULL DEFAULT 'none',
    UNIQUE(project_id, name)
);

INSERT INTO statuses (id, project_id, name, position, color, style)
    SELECT id, project_id, name, position, color,
       CASE WHEN style = 'completed' THEN 'checked' ELSE 'unchecked' END
    FROM statuses_backup;

DROP TABLE statuses_backup;
