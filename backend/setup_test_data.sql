-- Test Database Configuration
INSERT INTO database_configs (id, name, host, port, username, password, database_name, created_at, updated_at)
VALUES (
    'test-db-config-001',
    'Test Database',
    '127.0.0.1',
    3306,
    'root',
    '',
    'test_db',
    datetime('now'),
    datetime('now')
);

-- Test Task
INSERT INTO tasks (id, name, database_config_id, cron_schedule, compression_type, cleanup_days, is_active, created_at, updated_at)
VALUES (
    'test-task-001',
    'Daily Test Backup',
    'test-db-config-001',
    '0 2 * * *',
    'gzip',
    7,
    1,
    datetime('now'),
    datetime('now')
);

-- Production Database Configuration (example)
INSERT INTO database_configs (id, name, host, port, username, password, database_name, created_at, updated_at)
VALUES (
    'prod-db-config-001',
    'Production Database',
    'mysql.example.com',
    3306,
    'backup_user',
    'secure_password',
    'production_db',
    datetime('now'),
    datetime('now')
);

-- Production Task
INSERT INTO tasks (id, name, database_config_id, cron_schedule, compression_type, cleanup_days, is_active, created_at, updated_at)
VALUES (
    'prod-task-001',
    'Hourly Production Backup',
    'prod-db-config-001',
    '0 * * * *',
    'gzip',
    30,
    0, -- Disabled by default
    datetime('now'),
    datetime('now')
);
