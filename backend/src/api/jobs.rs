use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::models::{Job, CreateJobRequest, JobStatus};
use crate::services::progress_tracker::ProgressTracker;
use super::{ApiError, ApiResult, success_response, paginated_response};

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

    let mut sql = "SELECT * FROM jobs".to_string();
    let mut count_sql = "SELECT COUNT(*) as count FROM jobs".to_string();
    let mut conditions = Vec::new();
    
    if query.status.is_some() {
        conditions.push("status = ?");
    }
    
    if query.job_type.is_some() {
        conditions.push("job_type = ?");
    }
    
    if query.task_id.is_some() {
        conditions.push("task_id = ?");
    }
    
    if !conditions.is_empty() {
        let where_clause = format!(" WHERE {}", conditions.join(" AND "));
        sql.push_str(&where_clause);
        count_sql.push_str(&where_clause);
    }
    
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));

    let mut query_builder = sqlx::query_as(&sql);
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

    let jobs: Vec<Job> = query_builder.fetch_all(&pool).await?;
    let total: (i64,) = count_query_builder.fetch_one(&pool).await?;

    Ok(paginated_response(jobs, page, limit, total.0 as u64))
}

async fn get_job(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let job: Option<Job> = sqlx::query_as(
        "SELECT * FROM jobs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    match job {
        Some(job) => Ok(success_response(job)),
        None => Err(ApiError::NotFound("Job not found".to_string())),
    }
}

async fn create_job(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateJobRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Validate task exists if task_id is provided
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
        INSERT INTO jobs (id, task_id, job_type, status, progress, started_at, completed_at, error_message, log_output, backup_path, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&job.id)
    .bind(&job.task_id)
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
        "UPDATE jobs SET status = ?, completed_at = ? WHERE id = ?"
    )
    .bind(JobStatus::Cancelled.to_string())
    .bind(chrono::Utc::now())
    .bind(&id)
    .execute(&pool)
    .await?;

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
    let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backup".to_string());
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
    use crate::services::mydumper::MydumperService;
    
    // Get job from database
    let job: Option<Job> = sqlx::query_as(
        "SELECT * FROM jobs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    let job = job.ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;
    
    // For running jobs, try to get real-time progress from logs
    if job.status == "running" {
        let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backup".to_string());
        let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "data/logs".to_string());
        let mydumper_service = MydumperService::new(backup_dir, log_dir);
        
        match mydumper_service.get_job_progress_from_logs(&id).await {
            Ok((live_progress, _)) => {
                // Update job progress in database if we got a live update
                if live_progress > job.progress {
                    let _ = sqlx::query("UPDATE jobs SET progress = ? WHERE id = ?")
                        .bind(live_progress)
                        .bind(&id)
                        .execute(&pool)
                        .await;
                        
                    return Ok(success_response(serde_json::json!({
                        "job_id": id,
                        "progress": live_progress,
                        "status": job.status,
                        "updated_from_logs": true
                    })));
                }
            }
            Err(_) => {
                // Fall back to database progress
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
    let jobs: Vec<Job> = sqlx::query_as(
        "SELECT * FROM jobs WHERE status IN ('pending', 'running') ORDER BY created_at DESC"
    )
    .fetch_all(&pool)
    .await?;

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