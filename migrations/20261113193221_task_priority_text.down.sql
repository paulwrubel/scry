ALTER TABLE tasks
    ALTER COLUMN priority TYPE BIGINT
    USING CASE priority
        WHEN 'critical' THEN 1
        WHEN 'elevated' THEN 2
        ELSE 3
    END;
