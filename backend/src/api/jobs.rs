use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};

use crate::models::{Job, CreateJobRequest, JobStatus};
use crate::services::progress_tracker::ProgressTracker;
use super::{ApiError, ApiResult, success_response, paginated_response};

#[derive(Debug, Serialize)]
pub struct JobWithDatabaseInfo {
    #[serde(flatten)]
    pub job: Job,
    pub task_name: Option<String>,
    pub task_database_name: Option<String>,
    pub db_config_name: Option<String>,
    pub db_config_host: Option<String>,
    pub db_config_database_name: Option<String>,
}


#[derive(Deserialize)]
pub struct ListQuery {
    page: Option<u32>,
    limit: Option<u32>,
    status: Option<String>,
    job_type: Option<String>,
    task_id: Option<String>,
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(list_jobs).post(create_job))
        .route("/:id", get(get_job).delete(delete_job))
        .route("/:id/cancel", post(cancel_job))
        .route("/:id/logs", get(get_job_logs))
        .route("/:id/progress", get(get_job_progress))
        .route("/:id/detailed-progress", get(get_detailed_progress))
        .route("/active", get(list_active_jobs))
        .with_state(pool)
}

async fn list_jobs(
    State(pool): State<SqlitePool>,
    Query(query): Query<ListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    let offset = (page - 1) * limit;

    let mut sql = "SELECT j.*, t.name as task_name, t.database_name as task_database_name, dc.name as db_config_name, dc.host as db_config_host, dc.database_name as db_config_database_name FROM jobs j LEFT JOIN tasks t ON j.task_id = t.id LEFT JOIN database_configs dc ON t.database_config_id = dc.id".to_string();
    let mut count_sql = "SELECT COUNT(*) as count FROM jobs j LEFT JOIN tasks t ON j.task_id = t.id LEFT JOIN database_configs dc ON t.database_config_id = dc.id".to_string();
    let mut conditions = Vec::new();
    
    if query.status.is_some() {
        conditions.push("j.status = ?");
    }
    
    if query.job_type.is_some() {
        conditions.push("j.job_type = ?");
    }
    
    if query.task_id.is_some() {
        conditions.push("j.task_id = ?");
    }
    
    if !conditions.is_empty() {
        let where_clause = format!(" WHERE {}", conditions.join(" AND "));
        sql.push_str(&where_clause);
        count_sql.push_str(&where_clause);
    }
    
    sql.push_str(&format!(" ORDER BY j.created_at DESC LIMIT {} OFFSET {}", limit, offset));

    let mut query_builder = sqlx::query(&sql);
    let mut count_query_builder = sqlx::query_as(&count_sql);
    
    if let Some(ref status) = query.status {
        query_builder = query_builder.bind(status);
        count_query_builder = count_query_builder.bind(status);
    }
    
    if let Some(ref job_type) = query.job_type {
        query_builder = query_builder.bind(job_type);
        count_query_builder = count_query_builder.bind(job_type);
    }
    
    if let Some(ref task_id) = query.task_id {
        query_builder = query_builder.bind(task_id);
        count_query_builder = count_query_builder.bind(task_id);
    }

    let rows = query_builder.fetch_all(&pool).await?;
    let total: (i64,) = count_query_builder.fetch_one(&pool).await?;

    let mut jobs: Vec<JobWithDatabaseInfo> = rows.into_iter().map(|row| {
        JobWithDatabaseInfo {
            job: Job {
                id: row.get("id"),
                task_id: row.get("task_id"),
                used_database: row.get("used_database"),
                job_type: row.get("job_type"),
                status: row.get("status"),
                progress: row.get("progress"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                error_message: row.get("error_message"),
                log_output: row.get("log_output"),
                backup_path: row.get("backup_path"),
                created_at: row.get("created_at"),
            },
            task_name: row.get("task_name"),
            task_database_name: row.get("task_database_name"),
            db_config_name: row.get("db_config_name"),
            db_config_host: row.get("db_config_host"),
            db_config_database_name: row.get("db_config_database_name"),
        }
    }).collect();

    // Update progress for running jobs using the same logic as detailed progress
    for job in &mut jobs {
        if job.job.status == "running" || job.job.status == "compressing" {
            if let Some(log_output) = &job.job.log_output {
                if let Some(log_dir) = std::path::Path::new(log_output).parent() {
                    if let Some(log_dir_str) = log_dir.to_str() {
                        let progress_tracker = ProgressTracker::new(log_dir_str.to_string());
                        if let Ok(detailed_progress) = progress_tracker.load_detailed_progress(&job.job.id).await {
                            job.job.progress = detailed_progress.overall_progress as i32;
                        }
                    }
                }
            }
        }
    }

    Ok(paginated_response(jobs, page, limit, total.0 as u64))
}

async fn get_job(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let mut job: Option<Job> = sqlx::query_as(
        "SELECT * FROM jobs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    match job {
        Some(mut job) => {
            // Update progress for running jobs using the same logic as detailed progress
            if job.status == "running" || job.status == "compressing" {
                if let Some(log_output) = &job.log_output {
                    if let Some(log_dir) = std::path::Path::new(log_output).parent() {
                        if let Some(log_dir_str) = log_dir.to_str() {
                            let progress_tracker = ProgressTracker::new(log_dir_str.to_string());
                            if let Ok(detailed_progress) = progress_tracker.load_detailed_progress(&job.id).await {
                                job.progress = detailed_progress.overall_progress as i32;
                            }
                        }
                    }
                }
            }
            Ok(success_response(job))
        },
        None => Err(ApiError::NotFound("Job not found".to_string())),
    }
}

async fn create_job(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateJobRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Validate task exists if task_id is provided
    // Validate task_id if provided
    if let Some(ref task_id) = req.task_id {
        let task_exists: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM tasks WHERE id = ?"
        )
        .bind(task_id)
        .fetch_optional(&pool)
        .await?;

        if task_exists.is_none() {
            return Err(ApiError::BadRequest("Task not found".to_string()));
        }
    }

    let job = Job::new(req);

    sqlx::query(
        r#"
        INSERT INTO jobs (id, task_id, used_database, job_type, status, progress, started_at, completed_at, error_message, log_output, backup_path, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&job.id)
    .bind(&job.task_id)
    .bind(&job.used_database)
    .bind(&job.job_type)
    .bind(&job.status)
    .bind(&job.progress)
    .bind(&job.started_at)
    .bind(&job.completed_at)
    .bind(&job.error_message)
    .bind(&job.log_output)
    .bind(&job.backup_path)
    .bind(&job.created_at)
    .execute(&pool)
    .await?;

    Ok(success_response(job))
}

async fn delete_job(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    use std::path::Path as StdPath;
    
    // Check if job exists and is not running
    let job: Option<Job> = sqlx::query_as(
        "SELECT * FROM jobs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    let job = job.ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    // Don't allow deletion of running jobs
    if job.status().unwrap_or(JobStatus::Failed) == JobStatus::Running {
        return Err(ApiError::BadRequest("Cannot delete a running job. Cancel it first.".to_string()));
    }

    // Delete the log file if it exists
    if let Some(log_output) = &job.log_output {
        if StdPath::new(log_output).exists() {
            if let Err(e) = std::fs::remove_file(log_output) {
                tracing::warn!("Failed to delete log file {}: {}", log_output, e);
            }
        }
        
        // Also try to delete the log directory if it's empty
        if let Some(log_dir) = StdPath::new(log_output).parent() {
            if log_dir.exists() && log_dir.is_dir() {
                // Only delete if directory is empty (safe operation)
                if let Ok(entries) = std::fs::read_dir(log_dir) {
                    if entries.count() == 0 {
                        if let Err(e) = std::fs::remove_dir(log_dir) {
                            tracing::warn!("Failed to delete empty log directory {:?}: {}", log_dir, e);
                        }
                    } else {
                        // Directory is not empty, but we can still try to delete the entire directory
                        // since it belongs to this specific job
                        if let Err(e) = std::fs::remove_dir_all(log_dir) {
                            tracing::warn!("Failed to delete log directory {:?}: {}", log_dir, e);
                        }
                    }
                }
            }
        }
    }

    let result = sqlx::query("DELETE FROM jobs WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Job not found".to_string()));
    }

    // Log the deletion
    use crate::services::logging::LoggingService;
    use std::sync::Arc;
    let logging_service = LoggingService::new(Arc::new(pool.clone()));
    let _ = logging_service.log_system_with_entity(
        "job",
        &id,
        &format!("Job with status '{}' deleted", job.status),
        crate::models::log::LogLevel::Info
    ).await;

    Ok(success_response(serde_json::json!({"message": "Job deleted successfully"})))
}

async fn cancel_job(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let job: Option<Job> = sqlx::query_as(
        "SELECT * FROM jobs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    let job = job.ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    let current_status = job.status().unwrap_or(JobStatus::Failed);
    
    // Only allow cancellation of pending or running jobs
    if !matches!(current_status, JobStatus::Pending | JobStatus::Running) {
        return Err(ApiError::BadRequest("Job cannot be cancelled in its current state".to_string()));
    }

    sqlx::query(
        "UPDATE jobs SET status = ?, completed_at = ?, error_message = ? WHERE id = ?"
    )
    .bind(JobStatus::Cancelled.to_string())
    .bind(chrono::Utc::now())
    .bind("Job cancelled by user")
    .bind(&id)
    .execute(&pool)
    .await?;

    // Clean up backup directory if it exists
    let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string());
    let job_backup_dir = format!("{}/{}", backup_dir, id);
    if std::path::Path::new(&job_backup_dir).exists() {
        if let Err(e) = std::fs::remove_dir_all(&job_backup_dir) {
            tracing::warn!("Failed to remove backup directory {}: {}", job_backup_dir, e);
        } else {
            tracing::info!("Cleaned up backup directory: {}", job_backup_dir);
        }
    }

    // TODO: Send signal to actually cancel the running process

    Ok(success_response(serde_json::json!({
        "message": "Job cancelled successfully",
        "job_id": id
    })))
}

async fn get_job_logs(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    use crate::services::mydumper::MydumperService;
    
    // Create mydumper service instance
    let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string());
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "data/logs".to_string());
    let mydumper_service = MydumperService::new(backup_dir, log_dir);
    
    // Try to read logs from file first, then fallback to database
    match mydumper_service.read_job_logs(&id, &pool).await {
        Ok(logs) => {
            Ok(success_response(serde_json::json!({
                "job_id": id,
                "logs": logs
            })))
        }
        Err(_) => {
            // Fallback to database log_output
            let job: Option<Job> = sqlx::query_as(
                "SELECT log_output FROM jobs WHERE id = ?"
            )
            .bind(&id)
            .fetch_optional(&pool)
            .await?;

            let job = job.ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

            Ok(success_response(serde_json::json!({
                "job_id": id,
                "logs": job.log_output.unwrap_or_default()
            })))
        }
    }
}

async fn get_job_progress(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    use crate::services::progress_tracker::ProgressTracker;
    
    // Get job from database
    let job: Option<Job> = sqlx::query_as(
        "SELECT * FROM jobs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    let job = job.ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;
    
    // For running jobs, calculate progress on-the-fly from logs
    if job.status == "running" {
        if let Some(log_output) = &job.log_output {
            let log_dir = std::path::Path::new(log_output)
                .parent()
                .ok_or_else(|| ApiError::BadRequest("Invalid log path".to_string()))?
                .to_string_lossy()
                .to_string();

            let progress_tracker = ProgressTracker::new(log_dir);
            match progress_tracker.load_detailed_progress(&id).await {
                Ok(detailed_progress) => {
                    return Ok(success_response(serde_json::json!({
                        "job_id": id,
                        "progress": detailed_progress.overall_progress,
                        "status": job.status,
                        "updated_from_logs": true,
                        "total_tables": detailed_progress.total_tables,
                        "completed_tables": detailed_progress.completed_tables,
                        "in_progress_tables": detailed_progress.in_progress_tables,
                        "pending_tables": detailed_progress.pending_tables
                    })));
                }
                Err(_) => {
                    // Fall back to database progress
                }
            }
        }
    }
    
    Ok(success_response(serde_json::json!({
        "job_id": id,
        "progress": job.progress,
        "status": job.status,
        "updated_from_logs": false
    })))
}

async fn list_active_jobs(
    State(pool): State<SqlitePool>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let mut jobs: Vec<Job> = sqlx::query_as(
        "SELECT * FROM jobs WHERE status IN ('pending', 'running', 'compressing') ORDER BY created_at DESC"
    )
    .fetch_all(&pool)
    .await?;

    // Update progress for running jobs using the same logic as detailed progress
    for job in &mut jobs {
        if job.status == "running" || job.status == "compressing" {
            if let Some(log_output) = &job.log_output {
                if let Some(log_dir) = std::path::Path::new(log_output).parent() {
                    if let Some(log_dir_str) = log_dir.to_str() {
                        let progress_tracker = ProgressTracker::new(log_dir_str.to_string());
                        if let Ok(detailed_progress) = progress_tracker.load_detailed_progress(&job.id).await {
                            job.progress = detailed_progress.overall_progress as i32;
                        }
                    }
                }
            }
        }
    }

    Ok(success_response(jobs))
}

async fn get_detailed_progress(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Get job information
    let job: Job = sqlx::query_as(
        "SELECT * FROM jobs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    // Get log directory from job
    let log_output = job.log_output.as_ref()
        .ok_or_else(|| ApiError::BadRequest("Job has no log output".to_string()))?;
    
    let log_dir = std::path::Path::new(log_output)
        .parent()
        .ok_or_else(|| ApiError::BadRequest("Invalid log path".to_string()))?
        .to_string_lossy()
        .to_string();

    // Create progress tracker and load detailed progress
    let progress_tracker = ProgressTracker::new(log_dir);
    let detailed_progress = progress_tracker.load_detailed_progress(&id).await
        .map_err(|e| ApiError::InternalError(format!("Failed to load detailed progress: {}", e)))?;

    Ok(success_response(detailed_progress))
}