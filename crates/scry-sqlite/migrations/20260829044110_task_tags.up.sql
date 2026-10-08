-- Add up migration script here
ALTER TABLE tasks ADD COLUMN tags TEXT NOT NULL DEFAULT "";