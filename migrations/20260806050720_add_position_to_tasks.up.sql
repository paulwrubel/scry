ALTER TABLE tasks ADD COLUMN position INTEGER NOT NULL DEFAULT 0;

UPDATE tasks SET position = (
    SELECT COUNT(*)
    FROM tasks t2
    WHERE t2.state_id = tasks.state_id AND t2.id < tasks.id
);
