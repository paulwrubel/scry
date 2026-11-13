ALTER TABLE comments RENAME TO notes;

ALTER INDEX comments_task_id_idx RENAME TO notes_task_id_idx;

UPDATE changes SET entity_type = 'note' WHERE entity_type = 'comment';

UPDATE entity_tombstones SET entity_type = 'note' WHERE entity_type = 'comment';
