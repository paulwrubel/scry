ALTER TABLE tasks
    ALTER COLUMN priority TYPE TEXT
    USING CASE priority
        WHEN 1 THEN 'critical'
        WHEN 2 THEN 'elevated'
        ELSE 'normal'
    END;
