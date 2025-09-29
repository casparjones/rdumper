use axum::{
    extract::State,
    routing::get,
    Router,
};
use sqlx::SqlitePool;
use serde_json::json;

use crate::services::filesystem_backup::FilesystemBackupService;
use super::{ApiResult, success_response};

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/stats", get(get_dashboard_stats))
        .route("/recent-backups", get(get_recent_backups))
        .route("/next-tasks", get(get_next_tasks))
        .with_state(pool)
}

async fn get_dashboard_stats(
    State(pool): State<SqlitePool>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Get database configs count
    let db_configs_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM database_configs")
        .fetch_one(&pool)
        .await?;

    // Get tasks count
    let tasks_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
        .fetch_one(&pool)
        .await?;

    // Get active tasks count
    let active_tasks_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE is_active = true")
        .fetch_one(&pool)
        .await?;

    // Get total jobs count
    let total_jobs_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await?;

    // Get running jobs count (including compressing)
    let running_jobs_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM jobs WHERE status IN ('running', 'compressing')"
    )
        .fetch_one(&pool)
        .await?;

    // Get recent backups count (last 24 hours)
    let recent_backups_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM jobs WHERE job_type = 'backup' AND created_at > datetime('now', '-1 day')"
    )
        .fetch_one(&pool)
        .await?;

    // Get backup files count from filesystem
    let backup_base_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string());
    let filesystem_service = FilesystemBackupService::new(backup_base_dir);
    let backup_files = filesystem_service.scan_backups().await.unwrap_or_default();
    let backup_files_count = backup_files.len() as i64;

    Ok(success_response(json!({
        "databases": db_configs_count.0,
        "tasks": tasks_count.0,
        "active_tasks": active_tasks_count.0,
        "total_jobs": total_jobs_count.0,
        "running_jobs": running_jobs_count.0,
        "recent_backups": recent_backups_count.0,
        "backup_files": backup_files_count,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn get_recent_backups(
    State(pool): State<SqlitePool>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Get recent backup jobs
    let recent_jobs: Vec<serde_json::Value> = sqlx::query_as::<_, (String, String, String, String, String, i32, String, String, Option<String>)>(
        "SELECT id, job_type, status, created_at, started_at, progress, error_message, log_output, backup_path FROM jobs WHERE job_type = 'backup' ORDER BY created_at DESC LIMIT 5"
    )
        .fetch_all(&pool)
        .await?
        .into_iter()
        .map(|(id, job_type, status, created_at, started_at, progress, error_message, log_output, backup_path)| {
            json!({
                "id": id,
                "job_type": job_type,
                "status": status,
                "created_at": created_at,
                "started_at": started_at,
                "progress": progress,
                "error_message": error_message,
                "log_output": log_output,
                "backup_path": backup_path
            })
        })
        .collect();

    Ok(success_response(json!({
        "recent_backups": recent_jobs,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn get_next_tasks(
    State(pool): State<SqlitePool>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Get next 5 scheduled tasks
    let next_tasks: Vec<serde_json::Value> = sqlx::query_as::<_, (String, String, String, String, String, String, i32, bool, Option<String>, Option<String>)>(
        "SELECT t.id, t.name, t.cron_schedule, t.database_config_id, t.created_at, t.updated_at, t.cleanup_days, t.is_active, dc.name as db_name, dc.database_name FROM tasks t LEFT JOIN database_configs dc ON t.database_config_id = dc.id WHERE t.is_active = true ORDER BY t.created_at ASC LIMIT 5"
    )
        .fetch_all(&pool)
        .await?
        .into_iter()
        .map(|(id, name, schedule, database_config_id, created_at, updated_at, cleanup_days, is_active, db_name, database_name)| {
            // Calculate next run time display (simplified since we don't have next_run field)
            let next_run_display = "Scheduled".to_string();

            // Format schedule display
            let schedule_display = match schedule.as_str() {
                "0 2 * * *" => "Daily at 2:00 AM",
                "0 2 * * 0" => "Weekly on Sunday at 2:00 AM",
                "0 2 1 * *" => "Monthly on 1st at 2:00 AM",
                _ => &schedule
            };

            json!({
                "id": id,
                "name": name,
                "database": db_name.unwrap_or_else(|| database_name.unwrap_or_else(|| "Unknown".to_string())),
                "next_run": next_run_display,
                "schedule": schedule_display,
                "is_active": is_active,
                "cleanup_days": cleanup_days
            })
        })
        .collect();

    Ok(success_response(json!({
        "next_tasks": next_tasks,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
