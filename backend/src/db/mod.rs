use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;

pub async fn create_database_pool(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    info!("Running database migrations");

    // Create database_configs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS database_configs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            host TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 3306,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            database_name TEXT NOT NULL DEFAULT '',
            connection_status TEXT NOT NULL DEFAULT 'untested',
            last_tested TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create tasks table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            database_config_id TEXT NOT NULL,
            database_name TEXT,
            cron_schedule TEXT NOT NULL,
            compression_type TEXT NOT NULL DEFAULT 'gzip',
            cleanup_days INTEGER NOT NULL DEFAULT 30,
            use_non_transactional BOOLEAN NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (database_config_id) REFERENCES database_configs (id) ON DELETE CASCADE
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Add use_non_transactional column to existing tasks table if it doesn't exist
    sqlx::query(
        r#"
        ALTER TABLE tasks ADD COLUMN use_non_transactional BOOLEAN NOT NULL DEFAULT 0
        "#
    )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    // Add last_run and next_run columns to existing tasks table if they don't exist
    sqlx::query(
        r#"
        ALTER TABLE tasks ADD COLUMN last_run TEXT
        "#
    )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    sqlx::query(
        r#"
        ALTER TABLE tasks ADD COLUMN next_run TEXT
        "#
    )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    // Add connection_status and last_tested columns to existing database_configs table if they don't exist
    sqlx::query(
        r#"
        ALTER TABLE database_configs ADD COLUMN connection_status TEXT NOT NULL DEFAULT 'untested'
        "#
    )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    sqlx::query(
        r#"
        ALTER TABLE database_configs ADD COLUMN last_tested TEXT
        "#
    )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    // Check if database_configs_new exists (migration already done)
    let table_exists: Result<Option<i64>, _> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='database_configs_new'"
    )
    .fetch_one(pool)
    .await;

    if let Ok(Some(count)) = table_exists {
        if count > 0 {
            // Migration already done, just rename the table
            sqlx::query("ALTER TABLE database_configs_new RENAME TO database_configs")
                .execute(pool)
                .await
                .ok();
        }
    } else {
        // Migration not done yet, perform it
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS database_configs_new (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                host TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 3306,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                database_name TEXT NOT NULL DEFAULT '',
                connection_status TEXT NOT NULL DEFAULT 'untested',
                last_tested TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
            .execute(pool)
            .await
            .ok();

        // Copy data from old table to new table
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO database_configs_new 
            SELECT id, name, host, port, username, password, database_name, 
                   COALESCE(connection_status, 'untested'), last_tested, created_at, updated_at
            FROM database_configs
            "#
        )
            .execute(pool)
            .await
            .ok();

        // Drop old table and rename new table
        sqlx::query("DROP TABLE IF EXISTS database_configs")
            .execute(pool)
            .await
            .ok();

        sqlx::query("ALTER TABLE database_configs_new RENAME TO database_configs")
            .execute(pool)
            .await
            .ok();
    }

    // Add database_name column to existing tasks table if it doesn't exist
    sqlx::query(
        r#"
        ALTER TABLE tasks ADD COLUMN database_name TEXT
        "#
    )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    // Create jobs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            task_id TEXT,
            used_database TEXT,
            job_type TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            progress INTEGER NOT NULL DEFAULT 0,
            started_at TEXT,
            completed_at TEXT,
            error_message TEXT,
            log_output TEXT,
            backup_path TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE SET NULL
        )
        "#
    )
        .execute(pool)
        .await?;

    // Create backups table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS backups (
            id TEXT PRIMARY KEY,
            database_config_id TEXT NOT NULL,
            task_id TEXT,
            used_database TEXT,
            file_path TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            compression_type TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (database_config_id) REFERENCES database_configs (id) ON DELETE CASCADE,
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE SET NULL
        )
        "#
    )
        .execute(pool)
        .await?;

    // Create logs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS logs (
            id TEXT PRIMARY KEY,
            log_type TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            message TEXT NOT NULL,
            level TEXT NOT NULL DEFAULT 'info',
            metadata TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
        .execute(pool)
        .await?;

    // Add used_database column to existing jobs table if it doesn't exist
    sqlx::query(
        r#"
        ALTER TABLE jobs ADD COLUMN used_database TEXT
        "#
    )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    // Add used_database column to existing backups table if it doesn't exist
    sqlx::query(
        r#"
        ALTER TABLE backups ADD COLUMN used_database TEXT
        "#
    )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    info!("Database migrations completed successfully");
    Ok(())
}
