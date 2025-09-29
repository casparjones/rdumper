use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
    response::Response,
    body::Body,
};
use axum_extra::extract::Multipart;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::path::Path as StdPath;
use tracing::error;

use crate::models::{Backup, RestoreRequest, Job, CreateJobRequest, JobType};
use crate::services::FilesystemBackupService;
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
        .route("/", get(list_backups))
        .route("/upload", post(upload_backup))
        .route("/:id", get(get_backup).delete(delete_backup))
        .route("/:id/restore", post(restore_backup))
        .route("/:id/download", get(download_backup))
        .route("/:id/metadata", post(update_metadata))
        .route("/cleanup", post(cleanup_old_backups))
        .with_state(pool)
}

async fn list_backups(
    State(_pool): State<SqlitePool>,
    Query(query): Query<ListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    let _offset = (page - 1) * limit;

    // Initialize filesystem backup service
    let backup_service = FilesystemBackupService::new(
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string())
    );

    // Scan filesystem for backups
    let mut all_backups = backup_service.scan_backups().await
        .map_err(|e| ApiError::InternalError(format!("Failed to scan backups: {}", e)))?;

    // Apply filters
    if let Some(ref db_config_id) = query.database_config_id {
        all_backups.retain(|b| b.database_config_id == *db_config_id);
    }
    
    if let Some(ref task_id) = query.task_id {
        all_backups.retain(|b| b.task_id.as_ref() == Some(task_id));
    }

    let total = all_backups.len();
    
    // Apply pagination
    let start = ((page - 1) * limit) as usize;
    let end = std::cmp::min(start + limit as usize, total);
    let backups = if start < total {
        all_backups[start..end].to_vec()
    } else {
        Vec::new()
    };

    Ok(paginated_response(backups, page, limit, total as u64))
}

async fn get_backup(
    State(_pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Initialize filesystem backup service
    let backup_service = FilesystemBackupService::new(
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string())
    );

    // Scan filesystem for backups
    let backups = backup_service.scan_backups().await
        .map_err(|e| ApiError::InternalError(format!("Failed to scan backups: {}", e)))?;

    // Find backup by ID
    let backup = backups.into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| ApiError::NotFound("Backup not found".to_string()))?;

    // Load full metadata
    let _metadata = backup.load_metadata().await
        .map_err(|e| ApiError::InternalError(format!("Failed to load backup metadata: {}", e)))?;

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
        error!("Multipart field error: {}", e);
        ApiError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let field_name = field.name().unwrap_or("unknown");
        error!("Processing field: '{}'", field_name);
        
        match field_name {
            "file" => {
                if let Some(name) = field.file_name() {
                    filename = name.to_string();
                    error!("File name: '{}'", filename);
                }
                let data = field.bytes().await.map_err(|e| {
                    error!("Failed to read file bytes: {}", e);
                    ApiError::BadRequest(format!("Failed to read file data: {}", e))
                })?;
                file_data = data.to_vec();
                error!("File data size: {} bytes", file_data.len());
            }
            "database_config_id" => {
                let text = field.text().await.map_err(|e| {
                    error!("Failed to read database_config_id text: {}", e);
                    ApiError::BadRequest(format!("Failed to read database_config_id: {}", e))
                })?;
                database_config_id = text;
                error!("Database config ID: '{}'", database_config_id);
            }
            "compression_type" => {
                let text = field.text().await.map_err(|e| {
                    error!("Failed to read compression_type text: {}", e);
                    ApiError::BadRequest(format!("Failed to read compression_type: {}", e))
                })?;
                compression_type = text;
                error!("Compression type: '{}'", compression_type);
            }
            _ => {
                error!("Unknown field: '{}'", field_name);
            }
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

    // Create temporary file first
    let temp_dir = std::env::var("TEMP_DIR").unwrap_or_else(|_| "/tmp".to_string());
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
    
    let temp_filename = format!("uploaded_{}_{}.{}", 
        safe_filename.trim_end_matches(&format!(".{}", file_extension)),
        timestamp,
        file_extension
    );
    
    let temp_path = format!("{}/{}", temp_dir, temp_filename);

    // Write file to temporary location
    tokio::fs::write(&temp_path, &file_data).await.map_err(|e| {
        ApiError::InternalError(format!("Failed to write backup file: {}", e))
    })?;

    // Get database config for metadata
    let db_config: crate::models::DatabaseConfig = sqlx::query_as(
        "SELECT * FROM database_configs WHERE id = ?"
    )
    .bind(&database_config_id)
    .fetch_one(&pool)
    .await?;

    // Initialize filesystem backup service
    let backup_service = FilesystemBackupService::new(
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string())
    );

    // For uploaded files, we need to extract them first if they are archives
    let extract_dir = if filename.ends_with(".tar.gz") || filename.ends_with(".tar.zst") {
        // Extract the uploaded archive to a temporary directory
        let extract_path = format!("{}/extracted_{}", temp_dir, timestamp);
        std::fs::create_dir_all(&extract_path).map_err(|e| ApiError::InternalError(format!("Failed to create extract directory: {}", e)))?;
        
        let mut cmd = tokio::process::Command::new("tar");
        if filename.ends_with(".tar.gz") {
            cmd.args(&["-xzf", &temp_path, "-C", &extract_path]);
        } else {
            cmd.args(&["--zstd", "-xf", &temp_path, "-C", &extract_path]);
        }
        
        let status = cmd.status().await.map_err(|e| ApiError::InternalError(format!("Failed to execute tar command: {}", e)))?;
        if !status.success() {
            return Err(ApiError::InternalError("Failed to extract uploaded archive".to_string()));
        }
        
        extract_path
    } else {
        // For non-archive files, create a directory and copy the file
        let file_dir = format!("{}/file_{}", temp_dir, timestamp);
        std::fs::create_dir_all(&file_dir).map_err(|e| ApiError::InternalError(format!("Failed to create file directory: {}", e)))?;
        std::fs::copy(&temp_path, format!("{}/{}", file_dir, filename)).map_err(|e| ApiError::InternalError(format!("Failed to copy file: {}", e)))?;
        file_dir
    };

    // Create backup using new BackupProcess system
    let backup_id = uuid::Uuid::new_v4().to_string();
    let mut backup_process = backup_service.create_backup_process(&backup_id, &db_config, None).await
        .map_err(|e| ApiError::InternalError(format!("Failed to create backup process: {}", e)))?;
    
    // Copy files from extract_dir to tmp directory
    let tmp_dir = backup_process.tmp_dir().to_path_buf();
    std::fs::create_dir_all(&tmp_dir).map_err(|e| ApiError::InternalError(format!("Failed to create tmp directory: {}", e)))?;
    
    // Copy files from extract_dir to tmp_dir
    let mut entries = std::fs::read_dir(&extract_dir).map_err(|e| ApiError::InternalError(format!("Failed to read extract directory: {}", e)))?;
    while let Some(entry) = entries.next() {
        let entry = entry.map_err(|e| ApiError::InternalError(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().unwrap();
            std::fs::copy(&path, tmp_dir.join(filename)).map_err(|e| ApiError::InternalError(format!("Failed to copy file: {}", e)))?;
        }
    }
    
    // Complete the backup process
    backup_process.complete().await.map_err(|e| ApiError::InternalError(format!("Failed to complete backup: {}", e)))?;

    // Clean up temporary files and directories
    let _ = tokio::fs::remove_file(&temp_path).await;
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;

    Ok(success_response(serde_json::json!({
        "message": "Backup uploaded successfully",
        "backup_id": backup_id,
        "original_filename": filename
    })))
}

async fn delete_backup(
    State(_pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Initialize filesystem backup service
    let backup_service = FilesystemBackupService::new(
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string())
    );

    // Scan filesystem for backups
    let backups = backup_service.scan_backups().await
        .map_err(|e| ApiError::InternalError(format!("Failed to scan backups: {}", e)))?;

    // Find backup by ID
    let backup = backups.into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| ApiError::NotFound("Backup not found".to_string()))?;

    // Delete backup from filesystem
    backup_service.delete_backup(&backup).await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete backup: {}", e)))?;

    Ok(success_response(serde_json::json!({"message": "Backup deleted successfully"})))
}

async fn restore_backup(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<RestoreRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Initialize filesystem backup service
    let backup_service = FilesystemBackupService::new(
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string())
    );

    // Scan filesystem for backups
    let backups = backup_service.scan_backups().await
        .map_err(|e| ApiError::InternalError(format!("Failed to scan backups: {}", e)))?;

    // Find backup by ID
    let backup = backups.into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| ApiError::NotFound("Backup not found".to_string()))?;

    // Validate backup file exists
    if !StdPath::new(&backup.file_path).exists() {
        return Err(ApiError::BadRequest("Backup file no longer exists".to_string()));
    }

    // Load backup metadata to get database config info
    let metadata = backup.load_metadata().await
        .map_err(|e| ApiError::InternalError(format!("Failed to load backup metadata: {}", e)))?;

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
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string()),
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
    State(_pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Response<Body>, ApiError> {
    // Initialize filesystem backup service
    let backup_service = FilesystemBackupService::new(
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string())
    );

    // Scan filesystem for backups
    let backups = backup_service.scan_backups().await
        .map_err(|e| ApiError::InternalError(format!("Failed to scan backups: {}", e)))?;

    // Find backup by ID
    let backup = backups.into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| ApiError::NotFound("Backup not found".to_string()))?;

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
    State(_pool): State<SqlitePool>,
    Query(query): Query<serde_json::Value>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let days = query.get("days")
        .and_then(|v| v.as_u64())
        .unwrap_or(30) as i64;

    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days);

    // Initialize filesystem backup service
    let backup_service = FilesystemBackupService::new(
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string())
    );

    // Scan filesystem for backups
    let all_backups = backup_service.scan_backups().await
        .map_err(|e| ApiError::InternalError(format!("Failed to scan backups: {}", e)))?;

    // Filter old backups
    let old_backups: Vec<Backup> = all_backups.into_iter()
        .filter(|backup| {
            if let Ok(created_at) = chrono::DateTime::parse_from_rfc3339(&backup.created_at) {
                created_at.with_timezone(&chrono::Utc) < cutoff_date
            } else {
                false
            }
        })
        .collect();

    let mut deleted_count = 0;
    let mut failed_deletions = Vec::new();

    for backup in old_backups {
        match backup_service.delete_backup(&backup).await {
            Ok(_) => deleted_count += 1,
            Err(e) => {
                tracing::error!("Failed to delete backup {}: {}", backup.id, e);
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

#[derive(Deserialize)]
pub struct UpdateMetadataRequest {
    pub database_name: Option<String>,
    pub database_config_id: Option<String>,
    pub backup_type: Option<String>,
    pub compression_type: Option<String>,
}

async fn update_metadata(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(request): Json<UpdateMetadataRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Initialize filesystem backup service
    let backup_service = FilesystemBackupService::new(
        std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string())
    );

    // Find the backup
    let backups = backup_service.scan_backups().await
        .map_err(|e| ApiError::InternalError(format!("Failed to scan backups: {}", e)))?;
    
    let backup = backups.iter()
        .find(|b| b.id == id)
        .ok_or_else(|| ApiError::NotFound("Backup not found".to_string()))?;

    // Load current metadata
    let mut metadata = backup_service.load_backup_metadata(
        std::path::Path::new(&backup.meta_path)
    ).await
    .map_err(|e| ApiError::InternalError(format!("Failed to load metadata: {}", e)))?;

    // Update metadata fields
    if let Some(database_name) = request.database_name {
        metadata.database_name = database_name;
    }
    if let Some(database_config_id) = request.database_config_id {
        metadata.database_config_id = database_config_id;
    }
    if let Some(backup_type) = request.backup_type {
        metadata.backup_type = backup_type;
    }
    if let Some(compression_type) = request.compression_type {
        metadata.compression_type = compression_type;
    }

    // Save updated metadata
    backup_service.save_backup_metadata(&metadata).await
        .map_err(|e| ApiError::InternalError(format!("Failed to save metadata: {}", e)))?;

    Ok(success_response(serde_json::json!({
        "message": "Metadata updated successfully",
        "backup": metadata
    })))
}