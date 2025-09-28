use axum::{
    extract::{Path, Query, State, Multipart},
    routing::{delete, get, post},
    Json, Router,
    response::Response,
    body::Body,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::path::Path as StdPath;
use tracing::error;

use crate::models::{Backup, CreateBackupRequest, RestoreRequest, Job, CreateJobRequest, JobType};
use super::{ApiError, ApiResult, success_response, paginated_response};

#[derive(Deserialize)]
pub struct ListQuery {
    page: Option<u32>,
    limit: Option<u32>,
    database_config_id: Option<String>,
    task_id: Option<String>,
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(list_backups).post(create_backup))
        .route("/upload", post(upload_backup))
        .route("/:id", get(get_backup).delete(delete_backup))
        .route("/:id/restore", post(restore_backup))
        .route("/:id/download", get(download_backup))
        .route("/cleanup", post(cleanup_old_backups))
        .with_state(pool)
}

async fn list_backups(
    State(pool): State<SqlitePool>,
    Query(query): Query<ListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    let offset = (page - 1) * limit;

    let mut sql = "SELECT * FROM backups".to_string();
    let mut count_sql = "SELECT COUNT(*) as count FROM backups".to_string();
    let mut conditions = Vec::new();
    
    if query.database_config_id.is_some() {
        conditions.push("database_config_id = ?");
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
    
    if let Some(ref db_config_id) = query.database_config_id {
        query_builder = query_builder.bind(db_config_id);
        count_query_builder = count_query_builder.bind(db_config_id);
    }
    
    if let Some(ref task_id) = query.task_id {
        query_builder = query_builder.bind(task_id);
        count_query_builder = count_query_builder.bind(task_id);
    }

    let mut backups: Vec<Backup> = query_builder.fetch_all(&pool).await?;
    
    // Check if backup files still exist and update file sizes
    for backup in &mut backups {
        if let Ok(metadata) = std::fs::metadata(&backup.file_path) {
            if backup.file_size != metadata.len() as i64 {
                // Update file size in database if it has changed
                let _ = sqlx::query(
                    "UPDATE backups SET file_size = ? WHERE id = ?"
                )
                .bind(metadata.len() as i64)
                .bind(&backup.id)
                .execute(&pool)
                .await;
                
                backup.file_size = metadata.len() as i64;
            }
        }
    }
    
    let total: (i64,) = count_query_builder.fetch_one(&pool).await?;

    Ok(paginated_response(backups, page, limit, total.0 as u64))
}

async fn get_backup(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let backup: Option<Backup> = sqlx::query_as(
        "SELECT * FROM backups WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    match backup {
        Some(mut backup) => {
            // Check if file still exists and update size
            if let Ok(metadata) = std::fs::metadata(&backup.file_path) {
                backup.file_size = metadata.len() as i64;
            }
            Ok(success_response(backup))
        },
        None => Err(ApiError::NotFound("Backup not found".to_string())),
    }
}

async fn create_backup(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateBackupRequest>,
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

    // Validate that task exists if provided
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

    // Validate that backup file exists
    if !StdPath::new(&req.file_path).exists() {
        return Err(ApiError::BadRequest("Backup file does not exist".to_string()));
    }

    let backup = Backup::new(req);

    sqlx::query(
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
    .execute(&pool)
    .await?;

    Ok(success_response(backup))
}

async fn upload_backup(
    State(pool): State<SqlitePool>,
    mut multipart: Multipart,
) -> ApiResult<impl axum::response::IntoResponse> {
    let mut file_data = Vec::new();
    let mut filename = String::new();
    let mut database_config_id = String::new();
    let mut compression_type = "gzip".to_string();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        match field.name() {
            Some("file") => {
                if let Some(name) = field.file_name() {
                    filename = name.to_string();
                }
                let data = field.bytes().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read file data: {}", e))
                })?;
                file_data = data.to_vec();
            }
            Some("database_config_id") => {
                let text = field.text().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read database_config_id: {}", e))
                })?;
                database_config_id = text;
            }
            Some("compression_type") => {
                let text = field.text().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read compression_type: {}", e))
                })?;
                compression_type = text;
            }
            _ => {}
        }
    }

    if file_data.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    if database_config_id.is_empty() {
        return Err(ApiError::BadRequest("database_config_id is required".to_string()));
    }

    // Validate database config exists
    let db_config_exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM database_configs WHERE id = ?"
    )
    .bind(&database_config_id)
    .fetch_optional(&pool)
    .await?;

    if db_config_exists.is_none() {
        return Err(ApiError::BadRequest("Database configuration not found".to_string()));
    }

    // Create backup directory if it doesn't exist
    let backup_dir = std::env::var("BACKUP_BASE_DIR").unwrap_or_else(|_| "backend/data/backups".to_string());
    std::fs::create_dir_all(&backup_dir).map_err(|e| {
        ApiError::InternalError(format!("Failed to create backup directory: {}", e))
    })?;

    // Generate unique filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let file_extension = if filename.ends_with(".tar.gz") {
        "tar.gz"
    } else if filename.ends_with(".tar.zst") {
        "tar.zst"
    } else {
        "tar.gz" // Default to gzip
    };
    
    let safe_filename = filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();
    
    let backup_filename = format!("uploaded_{}_{}.{}", 
        safe_filename.trim_end_matches(&format!(".{}", file_extension)),
        timestamp,
        file_extension
    );
    
    let backup_path = format!("{}/{}", backup_dir, backup_filename);

    // Write file to disk
    tokio::fs::write(&backup_path, &file_data).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to write backup file: {}", e))
    })?;

    // Get file size
    let file_size = file_data.len() as i64;

    // Create backup record
    let backup_request = CreateBackupRequest {
        database_config_id,
        task_id: None, // Uploaded backups are not associated with tasks
        file_path: backup_path,
        file_size,
        compression_type,
    };

    let backup = Backup::new(backup_request);

    // Insert backup into database
    sqlx::query(
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
    .execute(&pool)
    .await?;

    Ok(success_response(serde_json::json!({
        "message": "Backup uploaded successfully",
        "backup": backup,
        "original_filename": filename
    })))
}

async fn delete_backup(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let backup: Option<Backup> = sqlx::query_as(
        "SELECT * FROM backups WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    let backup = backup.ok_or_else(|| ApiError::NotFound("Backup not found".to_string()))?;

    // Delete the actual backup file and related directories
    if StdPath::new(&backup.file_path).exists() {
        if let Err(e) = std::fs::remove_file(&backup.file_path) {
            tracing::warn!("Failed to delete backup file {}: {}", backup.file_path, e);
        }
    }

    // Note: Log files are deleted when the associated job is deleted
    // This ensures proper cleanup and avoids orphaned log files

    // Delete from database
    let result = sqlx::query("DELETE FROM backups WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Backup not found".to_string()));
    }

    Ok(success_response(serde_json::json!({"message": "Backup deleted successfully"})))
}

async fn restore_backup(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<RestoreRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Validate backup exists
    let backup: Option<Backup> = sqlx::query_as(
        "SELECT * FROM backups WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    let backup = backup.ok_or_else(|| ApiError::NotFound("Backup not found".to_string()))?;

    // Validate backup file exists
    if !StdPath::new(&backup.file_path).exists() {
        return Err(ApiError::BadRequest("Backup file no longer exists".to_string()));
    }

    // Validate target database config if specified
    let target_config_id = req.target_database_config_id.unwrap_or(backup.database_config_id.clone());
    let target_config_exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM database_configs WHERE id = ?"
    )
    .bind(&target_config_id)
    .fetch_optional(&pool)
    .await?;

    if target_config_exists.is_none() {
        return Err(ApiError::BadRequest("Target database configuration not found".to_string()));
    }

    // Create a restore job
    let job_request = CreateJobRequest {
        task_id: None,
        job_type: JobType::Restore,
        backup_path: Some(backup.file_path.clone()),
    };

    let job = Job::new(job_request);

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

    // Start the actual restore process using myloader
    let pool_clone = pool.clone();
    let mydumper_service = crate::services::MydumperService::new(
        std::env::var("BACKUP_BASE_DIR").unwrap_or_else(|_| "backend/data/backups".to_string()),
        std::env::var("LOG_BASE_DIR").unwrap_or_else(|_| "backend/data/logs".to_string()),
    );

    // Get target database config
    let target_config: crate::models::DatabaseConfig = sqlx::query_as(
        "SELECT * FROM database_configs WHERE id = ?"
    )
    .bind(&target_config_id)
    .fetch_one(&pool)
    .await?;

    // Generate new database name if requested
    let new_database_name = if let Some(new_name) = req.new_database_name {
        Some(new_name)
    } else if req.overwrite_existing {
        None
    } else {
        // Generate a new name with hash
        let hash = &backup.id[..5];
        Some(format!("{}_{}", target_config.database_name, hash))
    };

    // Clone job.id before moving into async closure
    let job_id = job.id.clone();
    let backup_id = backup.id.clone();
    let job_id_for_async = job_id.clone();

    // Start restore process asynchronously
    tokio::spawn(async move {
        // Update job status to running
        let _ = sqlx::query(
            "UPDATE jobs SET status = ?, started_at = ? WHERE id = ?"
        )
        .bind("running")
        .bind(chrono::Utc::now())
        .bind(&job_id_for_async)
        .execute(&pool_clone)
        .await;

        if let Err(e) = mydumper_service.restore_backup(
            &target_config,
            &backup.file_path,
            new_database_name.as_deref(),
            req.overwrite_existing,
        ).await {
            error!("Restore failed: {}", e);
            
            // Update job status to failed
            let _ = sqlx::query(
                "UPDATE jobs SET status = ?, error_message = ?, completed_at = ? WHERE id = ?"
            )
            .bind("failed")
            .bind(e.to_string())
            .bind(chrono::Utc::now())
            .bind(&job_id_for_async)
            .execute(&pool_clone)
            .await;
        } else {
            // Update job status to completed
            let _ = sqlx::query(
                "UPDATE jobs SET status = ?, completed_at = ?, progress = ? WHERE id = ?"
            )
            .bind("completed")
            .bind(chrono::Utc::now())
            .bind(100)
            .bind(&job_id_for_async)
            .execute(&pool_clone)
            .await;
        }
    });

    Ok(success_response(serde_json::json!({
        "message": "Restore job created successfully",
        "job_id": job_id,
        "backup_id": backup_id
    })))
}

async fn download_backup(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Response<Body>, ApiError> {
    let backup: Option<Backup> = sqlx::query_as(
        "SELECT * FROM backups WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let backup = backup.ok_or_else(|| ApiError::NotFound("Backup not found".to_string()))?;

    if !StdPath::new(&backup.file_path).exists() {
        return Err(ApiError::NotFound("Backup file not found on disk".to_string()));
    }

    // Read the file and return it as a download
    let file_content = tokio::fs::read(&backup.file_path).await
        .map_err(|_| ApiError::InternalError("Failed to read backup file".to_string()))?;

    let filename = backup.filename().unwrap_or("backup.tar.gz");
    let mime_type = if backup.file_path.ends_with(".tar.gz") {
        "application/gzip"
    } else if backup.file_path.ends_with(".tar.zst") {
        "application/zstd"
    } else {
        "application/octet-stream"
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", mime_type)
        .header("Content-Disposition", format!("attachment; filename=\"{}\"", filename))
        .header("Content-Length", backup.file_size.to_string())
        .body(Body::from(file_content))
        .unwrap())
}

async fn cleanup_old_backups(
    State(pool): State<SqlitePool>,
    Query(query): Query<serde_json::Value>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let days = query.get("days")
        .and_then(|v| v.as_u64())
        .unwrap_or(30) as i64;

    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days);

    // Get old backups to delete
    let old_backups: Vec<Backup> = sqlx::query_as(
        "SELECT * FROM backups WHERE created_at < ?"
    )
    .bind(&cutoff_date)
    .fetch_all(&pool)
    .await?;

    let mut deleted_count = 0;
    let mut failed_deletions = Vec::new();

    for backup in old_backups {
        // Delete the actual file
        if StdPath::new(&backup.file_path).exists() {
            if let Err(e) = std::fs::remove_file(&backup.file_path) {
                tracing::warn!("Failed to delete backup file {}: {}", backup.file_path, e);
                failed_deletions.push(backup.id.clone());
                continue;
            }
        }

        // Delete from database
        let result = sqlx::query("DELETE FROM backups WHERE id = ?")
            .bind(&backup.id)
            .execute(&pool)
            .await;

        match result {
            Ok(_) => deleted_count += 1,
            Err(e) => {
                tracing::error!("Failed to delete backup {} from database: {}", backup.id, e);
                failed_deletions.push(backup.id);
            }
        }
    }

    Ok(success_response(serde_json::json!({
        "message": format!("Cleanup completed. {} backups deleted.", deleted_count),
        "deleted_count": deleted_count,
        "failed_deletions": failed_deletions,
        "cutoff_date": cutoff_date.to_rfc3339()
    })))
}