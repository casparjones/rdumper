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
            database_name TEXT NOT NULL,
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

    // Create jobs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            task_id TEXT,
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

    info!("Database migrations completed successfully");
    Ok(())
}
