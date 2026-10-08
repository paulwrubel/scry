-- Add up migration script here
ALTER TABLE tasks ADD COLUMN priority INTEGER NOT NULL DEFAULT 3;
ALTER TABLE projects ADD COLUMN show_priority BOOLEAN NOT NULL DEFAULT false;
