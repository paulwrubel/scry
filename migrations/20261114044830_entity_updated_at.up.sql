ALTER TABLE projects ADD COLUMN updated_at TIMESTAMPTZ;
UPDATE projects SET updated_at = created_at;
ALTER TABLE projects ALTER COLUMN updated_at SET NOT NULL;

ALTER TABLE sections ADD COLUMN updated_at TIMESTAMPTZ;
UPDATE sections SET updated_at = created_at;
ALTER TABLE sections ALTER COLUMN updated_at SET NOT NULL;

ALTER TABLE tasks ADD COLUMN updated_at TIMESTAMPTZ;
UPDATE tasks SET updated_at = created_at;
ALTER TABLE tasks ALTER COLUMN updated_at SET NOT NULL;

ALTER TABLE comments ADD COLUMN updated_at TIMESTAMPTZ;
UPDATE comments SET updated_at = created_at;
ALTER TABLE comments ALTER COLUMN updated_at SET NOT NULL;

ALTER TABLE automations ADD COLUMN updated_at TIMESTAMPTZ;
UPDATE automations SET updated_at = created_at;
ALTER TABLE automations ALTER COLUMN updated_at SET NOT NULL;
