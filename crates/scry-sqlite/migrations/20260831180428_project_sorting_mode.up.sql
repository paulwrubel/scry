-- Add up migration script here
ALTER TABLE projects ADD COLUMN task_sorting_mode TEXT NOT NULL DEFAULT "alphabetical";
