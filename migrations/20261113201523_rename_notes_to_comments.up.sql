ALTER TABLE notes RENAME TO comments;

ALTER INDEX notes_task_id_idx RENAME TO comments_task_id_idx;

UPDATE changes SET entity_type = 'comment' WHERE entity_type = 'note';

UPDATE entity_tombstones SET entity_type = 'comment' WHERE entity_type = 'note';
