-- Store the active project as a project id instead of a name, so renames
-- cannot invalidate the pointer. Resolve any previously stored name to its
-- id; fall back to the default project (id 1) if it no longer resolves.
UPDATE config
SET value = COALESCE(
    CAST((SELECT id FROM projects WHERE name = config.value) AS TEXT),
    '1'
)
WHERE key = 'active_project';
