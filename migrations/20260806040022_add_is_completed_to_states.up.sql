ALTER TABLE states ADD COLUMN is_completed BOOLEAN NOT NULL DEFAULT 0;
UPDATE states SET is_completed = 1 WHERE name = 'done';
ALTER TABLE tasks DROP COLUMN completed_at;
