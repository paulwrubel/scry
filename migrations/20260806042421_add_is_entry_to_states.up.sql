ALTER TABLE states ADD COLUMN is_entry BOOLEAN NOT NULL DEFAULT 0;
UPDATE states SET is_entry = 1 WHERE name = 'todo';
