ALTER TABLE tasks ADD COLUMN IF NOT EXISTS closed_reason TEXT;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS closed_at TIMESTAMPTZ;

-- Before close reasons existed, a closed task had exactly one meaning: completed.
UPDATE tasks
SET closed_reason = 'completed'
WHERE state = 'closed' AND closed_reason IS NULL;

-- Recover the close time from the changes log, which records every state
-- transition. Tasks older than the log fall back to their last update.
UPDATE tasks t
SET closed_at = COALESCE(
    (
        SELECT c.created_at
        FROM changes c
        WHERE c.entity_type = 'task'
          AND c.entity_id = t.id
          AND c.field_values->>'state' = 'closed'
        ORDER BY c.seq DESC
        LIMIT 1
    ),
    t.updated_at
)
WHERE t.state = 'closed' AND t.closed_at IS NULL;
