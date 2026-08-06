ALTER TABLE tasks ADD COLUMN completed_at TIMESTAMP;
ALTER TABLE states DROP COLUMN is_completed;
