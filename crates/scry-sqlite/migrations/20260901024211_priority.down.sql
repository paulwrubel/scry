-- Add down migration script here
ALTER TABLE tasks DROP COLUMN priority;
ALTER TABLE projects DROP COLUMN show_priority;
