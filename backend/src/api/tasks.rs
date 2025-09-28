use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::models::{Task, CreateTaskRequest, UpdateTaskRequest};
use super::{ApiError, ApiResult, success_response, paginated_response};

#[derive(Deserialize)]
pub struct ListQuery {
    page: Option<u32>,
    limit: Option<u32>,
    database_config_id: Option<String>,
    is_active: Option<bool>,
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/:id", get(get_task).put(update_task).delete(delete_task))
        .route("/:id/run", post(run_task_now))
        .route("/:id/toggle", post(toggle_task_status))
        .with_state(pool)
}

async fn list_tasks(
    State(pool): State<SqlitePool>,
    Query(query): Query<ListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    let offset = (page - 1) * limit;

    let mut sql = "SELECT * FROM tasks".to_string();
    let mut count_sql = "SELECT COUNT(*) as count FROM tasks".to_string();
    let mut conditions = Vec::new();
    
    if query.database_config_id.is_some() {
        conditions.push("database_config_id = ?");
    }
    
    if query.is_active.is_some() {
        conditions.push("is_active = ?");
    }
    
    if !conditions.is_empty() {
        let where_clause = format!(" WHERE {}", conditions.join(" AND "));
        sql.push_str(&where_clause);
        count_sql.push_str(&where_clause);
    }
    
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));

    let mut query_builder = sqlx::query_as(&sql);
    let mut count_query_builder = sqlx::query_as(&count_sql);
    
    if let Some(ref db_config_id) = query.database_config_id {
        query_builder = query_builder.bind(db_config_id);
        count_query_builder = count_query_builder.bind(db_config_id);
    }
    
    if let Some(is_active) = query.is_active {
        query_builder = query_builder.bind(is_active);
        count_query_builder = count_query_builder.bind(is_active);
    }

    let tasks: Vec<Task> = query_builder.fetch_all(&pool).await?;
    let total: (i64,) = count_query_builder.fetch_one(&pool).await?;

    Ok(paginated_response(tasks, page, limit, total.0 as u64))
}

async fn get_task(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let task: Option<Task> = sqlx::query_as(
        "SELECT * FROM tasks WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    match task {
        Some(task) => Ok(success_response(task)),
        None => Err(ApiError::NotFound("Task not found".to_string())),
    }
}

async fn create_task(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateTaskRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Validate that database config exists
    let db_config_exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM database_configs WHERE id = ?"
    )
    .bind(&req.database_config_id)
    .fetch_optional(&pool)
    .await?;

    if db_config_exists.is_none() {
        return Err(ApiError::BadRequest("Database configuration not found".to_string()));
    }

    // Validate cron schedule format (basic validation)
    if req.cron_schedule.split_whitespace().count() != 5 {
        return Err(ApiError::BadRequest("Invalid cron schedule format. Expected: 'min hour day month weekday'".to_string()));
    }

    let task = Task::new(req);

    sqlx::query(
        r#"
        INSERT INTO tasks (id, name, database_config_id, cron_schedule, compression_type, cleanup_days, is_active, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&task.id)
    .bind(&task.name)
    .bind(&task.database_config_id)
    .bind(&task.cron_schedule)
    .bind(&task.compression_type)
    .bind(&task.cleanup_days)
    .bind(&task.is_active)
    .bind(&task.created_at)
    .bind(&task.updated_at)
    .execute(&pool)
    .await?;

    Ok(success_response(task))
}

async fn update_task(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let mut task: Task = sqlx::query_as(
        "SELECT * FROM tasks WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    // Validate cron schedule if provided
    if let Some(cron_schedule) = &req.cron_schedule {
        if cron_schedule.split_whitespace().count() != 5 {
            return Err(ApiError::BadRequest("Invalid cron schedule format. Expected: 'min hour day month weekday'".to_string()));
        }
    }

    task.update(req);

    sqlx::query(
        r#"
        UPDATE tasks 
        SET name = ?, cron_schedule = ?, compression_type = ?, cleanup_days = ?, is_active = ?, updated_at = ?
        WHERE id = ?
        "#
    )
    .bind(&task.name)
    .bind(&task.cron_schedule)
    .bind(&task.compression_type)
    .bind(&task.cleanup_days)
    .bind(&task.is_active)
    .bind(&task.updated_at)
    .bind(&task.id)
    .execute(&pool)
    .await?;

    Ok(success_response(task))
}

async fn delete_task(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Task not found".to_string()));
    }

    Ok(success_response(serde_json::json!({"message": "Task deleted successfully"})))
}

async fn run_task_now(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    use crate::services::mydumper::MydumperService;
    use crate::models::{CreateJobRequest, JobType};
    
    // Get the task
    let task: Task = sqlx::query_as(
        "SELECT * FROM tasks WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    // Get the database config for this task
    let db_config: crate::models::DatabaseConfig = sqlx::query_as(
        "SELECT * FROM database_configs WHERE id = ?"
    )
    .bind(&task.database_config_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Database configuration not found".to_string()))?;

    // Create a new job for this task execution
    let job_request = CreateJobRequest {
        task_id: Some(task.id.clone()),
        job_type: JobType::Backup,
        backup_path: None,
    };
    
    let job = crate::models::Job::new(job_request);
    let job_id = job.id.clone();

    // Insert the job into the database
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

    // Initialize mydumper service
    let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backup".to_string());
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "data/logs".to_string());
    let mydumper_service = MydumperService::new(backup_dir, log_dir);

    // Clone job_id for the response before moving it
    let response_job_id = job_id.clone();
    
    // Spawn the backup task asynchronously
    let task_clone = task.clone();
    let db_config_clone = db_config.clone();
    let pool_clone = pool.clone();
    
    tokio::spawn(async move {
        let result = mydumper_service
            .create_backup_with_progress(&db_config_clone, &task_clone, job_id.clone(), &pool_clone)
            .await;

        match result {
            Ok(backup_path) => {
                // Create backup record
                let backup_request = crate::models::CreateBackupRequest {
                    database_config_id: db_config_clone.id.clone(),
                    task_id: Some(task_clone.id.clone()),
                    file_path: backup_path.clone(),
                    file_size: std::fs::metadata(&backup_path)
                        .map(|m| m.len() as i64)
                        .unwrap_or(0),
                    compression_type: task_clone.compression_type.clone(),
                };

                let backup = crate::models::Backup::new(backup_request);

                // Insert backup record
                if let Err(e) = sqlx::query(
                    r#"
                    INSERT INTO backups (id, database_config_id, task_id, file_path, file_size, compression_type, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    "#
                )
                .bind(&backup.id)
                .bind(&backup.database_config_id)
                .bind(&backup.task_id)
                .bind(&backup.file_path)
                .bind(&backup.file_size)
                .bind(&backup.compression_type)
                .bind(&backup.created_at)
                .execute(&pool_clone)
                .await {
                    tracing::error!("Failed to create backup record: {}", e);
                }

                // Update job as completed
                let _ = sqlx::query("UPDATE jobs SET status = ?, completed_at = ?, progress = ?, backup_path = ? WHERE id = ?")
                    .bind("completed")
                    .bind(chrono::Utc::now())
                    .bind(100)
                    .bind(&backup_path)
                    .bind(&job_id)
                    .execute(&pool_clone)
                    .await;

                tracing::info!("Backup task {} completed successfully", task_clone.id);
            }
            Err(e) => {
                tracing::error!("Backup job {} failed: {}", job_id, e);
                // Update job status to failed
                let _ = sqlx::query("UPDATE jobs SET status = ?, error_message = ?, completed_at = ? WHERE id = ?")
                    .bind("failed")
                    .bind(e.to_string())
                    .bind(chrono::Utc::now())
                    .bind(&job_id)
                    .execute(&pool_clone)
                    .await;
            }
        }
    });

    Ok(success_response(serde_json::json!({
        "message": "Task execution started successfully",
        "job_id": response_job_id,
        "task_name": task.name,
        "database": db_config.name,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn toggle_task_status(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let task: Task = sqlx::query_as(
        "SELECT * FROM tasks WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Task not found".to_string()))?;

    let new_status = !task.is_active;

    sqlx::query(
        "UPDATE tasks SET is_active = ?, updated_at = ? WHERE id = ?"
    )
    .bind(new_status)
    .bind(chrono::Utc::now())
    .bind(&id)
    .execute(&pool)
    .await?;

    Ok(success_response(serde_json::json!({
        "message": format!("Task {} successfully", if new_status { "enabled" } else { "disabled" }),
        "is_active": new_status
    })))
}