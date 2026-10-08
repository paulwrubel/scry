-- Convert the active project id back to its project name.
UPDATE config
SET value = COALESCE(
    (SELECT name FROM projects WHERE id = CAST(config.value AS INTEGER)),
    'default'
)
WHERE key = 'active_project';
