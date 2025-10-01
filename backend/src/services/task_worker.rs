use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use sqlx::SqlitePool;
use tracing::{info, warn, error};
use chrono::{DateTime, Utc};
use crate::models::{Task, Job, JobType, JobStatus, CreateJobRequest, DatabaseConfig, LogLevel};
use crate::services::{MydumperService, LoggingService};

#[derive(Debug, Clone)]
pub struct WorkerStatus {
    pub is_running: bool,
    pub last_tick: Option<DateTime<Utc>>,
    pub total_ticks: u64,
    pub tasks_executed: u64,
}

pub struct TaskWorker {
    db_pool: Arc<SqlitePool>,
    status: Arc<Mutex<WorkerStatus>>,
}

impl TaskWorker {
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self {
            db_pool,
            status: Arc::new(Mutex::new(WorkerStatus {
                is_running: false,
                last_tick: None,
                total_ticks: 0,
                tasks_executed: 0,
            })),
        }
    }

    pub fn get_status(&self) -> WorkerStatus {
        self.status.lock().unwrap().clone()
    }

    /// Start the background worker that runs every minute
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting task worker...");
        
        let logging_service = LoggingService::new(self.db_pool.clone());
        let _ = logging_service.log_worker("Task worker started", LogLevel::Info).await;
        
        // Mark worker as running
        {
            let mut status = self.status.lock().unwrap();
            status.is_running = true;
        }
        
        loop {
            // Update last tick time
            {
                let mut status = self.status.lock().unwrap();
                status.last_tick = Some(Utc::now());
                status.total_ticks += 1;
            }
            
            if let Err(e) = self.check_and_execute_tasks().await {
                error!("Error in task worker: {}", e);
            }
            
            // Run cleanup tasks every hour (every 60 ticks)
            let should_run_cleanup = {
                let status = self.status.lock().unwrap();
                status.total_ticks % 60 == 0
            };
            
            if should_run_cleanup {
                if let Err(e) = self.run_cleanup_tasks().await {
                    error!("Error in cleanup tasks: {}", e);
                }
            }
            
            // Sleep for 1 minute
            sleep(Duration::from_secs(60)).await;
        }
    }

    /// Check all active tasks and execute them if their time has come
    async fn check_and_execute_tasks(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all active tasks
        let tasks = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE is_active = true"
        )
        .fetch_all(&*self.db_pool)
        .await?;

        let mut executed_count = 0;
        for task in tasks {
            if task.should_run_now() {
                let task_id = task.id.clone();
                if let Err(e) = self.execute_task(task).await {
                    error!("Failed to execute task {}: {}", task_id, e);
                } else {
                    executed_count += 1;
                }
            }
        }
        
        // Update tasks executed count
        {
            let mut status = self.status.lock().unwrap();
            status.tasks_executed += executed_count;
        }

        Ok(())
    }

    /// Execute a single task
    async fn execute_task(&self, mut task: Task) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Executing task: {} ({})", task.name, task.id);
        
        let logging_service = LoggingService::new(self.db_pool.clone());
        let _ = logging_service.log_task(&task.id, &format!("Task '{}' started", task.name), LogLevel::Info).await;

        // Check if there's already a running job for this task
        let running_job = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE task_id = ? AND status IN ('pending', 'running')"
        )
        .bind(&task.id)
        .fetch_optional(&*self.db_pool)
        .await?;

        if let Some(_running_job) = running_job {
            warn!("Task {} already has a running job, creating cancelled job", task.id);
            let _ = logging_service.log_task(&task.id, "Task cancelled - previous job still running", LogLevel::Warn).await;
            
            // Get database info for this task
            let db_config: DatabaseConfig = sqlx::query_as(
                "SELECT * FROM database_configs WHERE id = ?"
            )
            .bind(&task.database_config_id)
            .fetch_optional(&*self.db_pool)
            .await?
            .ok_or_else(|| "Database configuration not found".to_string())?;

            let database_name = match &task.database_name {
                Some(db_name) => db_name.clone(),
                None => {
                    match db_config.get_database_name() {
                        Some(db_name) => db_name.clone(),
                        None => {
                            error!("No database name specified for task {} and config has no default database", task.id);
                            let _ = logging_service.log_task(&task.id, "No database name specified for task and config has no default database", LogLevel::Error).await;
                            return Ok(());
                        }
                    }
                }
            };
            let used_database = format!("{}/{}", db_config.name, database_name);

            // Create a cancelled job with error message
            let cancelled_job = Job::new(CreateJobRequest {
                task_id: Some(task.id.clone()),
                used_database: Some(used_database),
                job_type: JobType::Backup,
                backup_path: None,
            });

            let mut cancelled_job = cancelled_job;
            cancelled_job.status = JobStatus::Cancelled.to_string();
            cancelled_job.error_message = Some("Previous task is still running".to_string());
            cancelled_job.completed_at = Some(chrono::Utc::now());

            sqlx::query(
                "INSERT INTO jobs (id, task_id, used_database, job_type, status, progress, started_at, completed_at, error_message, log_output, backup_path, created_at) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&cancelled_job.id)
            .bind(&cancelled_job.task_id)
            .bind(&cancelled_job.used_database)
            .bind(&cancelled_job.job_type)
            .bind(&cancelled_job.status)
            .bind(&cancelled_job.progress)
            .bind(&cancelled_job.started_at)
            .bind(&cancelled_job.completed_at)
            .bind(&cancelled_job.error_message)
            .bind(&cancelled_job.log_output)
            .bind(&cancelled_job.backup_path)
            .bind(&cancelled_job.created_at)
            .execute(&*self.db_pool)
            .await?;

            // Update task's next run time
            task.update_next_run()?;
            sqlx::query(
                "UPDATE tasks SET next_run = ?, updated_at = ? WHERE id = ?"
            )
            .bind(&task.next_run)
            .bind(&task.updated_at)
            .bind(&task.id)
            .execute(&*self.db_pool)
            .await?;

            return Ok(());
        }

        // Get database info for this task
        let db_config: DatabaseConfig = sqlx::query_as(
            "SELECT * FROM database_configs WHERE id = ?"
        )
        .bind(&task.database_config_id)
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| "Database configuration not found".to_string())?;

        let database_name = match &task.database_name {
            Some(db_name) => db_name.clone(),
            None => {
                match db_config.get_database_name() {
                    Some(db_name) => db_name.clone(),
                    None => {
                        error!("No database name specified for task {} and config has no default database", task.id);
                        let _ = logging_service.log_task(&task.id, "No database name specified for task and config has no default database", LogLevel::Error).await;
                        return Ok(());
                    }
                }
            }
        };
        let used_database = format!("{}/{}", db_config.name, database_name);

        // Create a new job for this task
        let job_request = CreateJobRequest {
            task_id: Some(task.id.clone()),
            used_database: Some(used_database),
            job_type: JobType::Backup,
            backup_path: None,
        };

        let job = Job::new(job_request);

        // Insert the job into database
        sqlx::query(
            "INSERT INTO jobs (id, task_id, used_database, job_type, status, progress, started_at, completed_at, error_message, log_output, backup_path, created_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
        .execute(&*self.db_pool)
        .await?;

        info!("Created job {} for task {}", job.id, task.id);
        let _ = logging_service.log_job(&job.id, &format!("Job created for task '{}'", task.name), LogLevel::Info).await;

        // Get the database config for this task
        let db_config: DatabaseConfig = sqlx::query_as(
            "SELECT * FROM database_configs WHERE id = ?"
        )
        .bind(&task.database_config_id)
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| "Database configuration not found".to_string())?;

        // Start the backup process asynchronously
        let db_pool = self.db_pool.clone();
        let job_id = job.id.clone();
        let task_clone = task.clone();
        let db_config_clone = db_config.clone();

        tokio::spawn(async move {
            let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "data/backups".to_string());
            let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "data/logs".to_string());
            let mydumper_service = MydumperService::new(backup_dir, log_dir);
            let logging_service = LoggingService::new(db_pool.clone());

            // Determine the database name to use
            let database_name = match &task_clone.database_name {
                Some(db_name) => db_name.clone(),
                None => {
                    // Use the database name from the config, or fail if none specified
                    match db_config_clone.get_database_name() {
                        Some(db_name) => db_name.clone(),
                        None => {
                            error!("No database name specified for task {} and config has no default database", task_clone.id);
                            let _ = logging_service.log_job(&job_id, "No database name specified for task and config has no default database", LogLevel::Error).await;
                            
                            // Update job as failed
                            let _ = sqlx::query("UPDATE jobs SET status = ?, completed_at = ?, error_message = ? WHERE id = ?")
                                .bind("failed")
                                .bind(chrono::Utc::now())
                                .bind("No database name specified for task and config has no default database")
                                .bind(&job_id)
                                .execute(&*db_pool)
                                .await;
                            return;
                        }
                    }
                }
            };

            let result = mydumper_service
                .create_backup_with_progress(&db_config_clone, &database_name, &task_clone, job_id.clone(), &db_pool)
                .await;

            match result {
                Ok(backup_file_path) => {
                    info!("Backup created successfully: {}", backup_file_path);
                    let _ = logging_service.log_job(&job_id, &format!("Backup completed successfully: {}", backup_file_path), LogLevel::Info).await;
                    
                    // Update job as completed
                    let _ = sqlx::query("UPDATE jobs SET status = ?, completed_at = ?, progress = ?, backup_path = ? WHERE id = ?")
                        .bind("completed")
                        .bind(chrono::Utc::now())
                        .bind(100)
                        .bind(&backup_file_path)
                        .bind(&job_id)
                        .execute(&*db_pool)
                        .await;
                }
                Err(e) => {
                    error!("Backup job {} failed: {}", job_id, e);
                    let _ = logging_service.log_job(&job_id, &format!("Backup failed: {}", e), LogLevel::Error).await;
                    
                    // Update job status to failed
                    let _ = sqlx::query("UPDATE jobs SET status = ?, error_message = ?, completed_at = ? WHERE id = ?")
                        .bind("failed")
                        .bind(e.to_string())
                        .bind(chrono::Utc::now())
                        .bind(&job_id)
                        .execute(&*db_pool)
                        .await;
                }
            }
        });

        // Update task's last_run and next_run
        task.mark_executed()?;
        sqlx::query(
            "UPDATE tasks SET last_run = ?, next_run = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&task.last_run)
        .bind(&task.next_run)
        .bind(&task.updated_at)
        .bind(&task.id)
        .execute(&*self.db_pool)
        .await?;

        info!("Updated task {} - last_run: {:?}, next_run: {:?}", 
              task.id, task.last_run, task.next_run);

        Ok(())
    }

    /// Run cleanup tasks (logs cleanup)
    async fn run_cleanup_tasks(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Running cleanup tasks...");
        
        let logging_service = LoggingService::new(self.db_pool.clone());
        
        // Clean up old logs (older than 14 days)
        match logging_service.cleanup_old_logs(14).await {
            Ok(deleted_count) => {
                if deleted_count > 0 {
                    info!("Cleaned up {} old log entries", deleted_count);
                    let _ = logging_service.log_worker(
                        &format!("Cleaned up {} old log entries", deleted_count), 
                        LogLevel::Info
                    ).await;
                } else {
                    info!("No old logs to clean up");
                }
            }
            Err(e) => {
                error!("Failed to clean up old logs: {}", e);
                let _ = logging_service.log_worker(
                    &format!("Failed to clean up old logs: {}", e), 
                    LogLevel::Error
                ).await;
            }
        }

        // Clean up old backups based on task configuration
        match self.cleanup_old_backups().await {
            Ok(deleted_count) => {
                if deleted_count > 0 {
                    info!("Cleaned up {} old backup files", deleted_count);
                    let _ = logging_service.log_worker(
                        &format!("Cleaned up {} old backup files", deleted_count), 
                        LogLevel::Info
                    ).await;
                } else {
                    info!("No old backups to clean up");
                }
            }
            Err(e) => {
                error!("Failed to clean up old backups: {}", e);
                let _ = logging_service.log_worker(
                    &format!("Failed to clean up old backups: {}", e), 
                    LogLevel::Error
                ).await;
            }
        }

        Ok(())
    }

    /// Clean up old backups based on task configuration
    async fn cleanup_old_backups(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        use std::path::Path;
        use tokio::fs;
        use chrono::Utc;
        
        let backup_dir = "./data/backups";
        if !Path::new(backup_dir).exists() {
            return Ok(0);
        }

        let mut deleted_count = 0u64;

        // Get all tasks with their cleanup_days configuration
        let tasks = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE is_active = true AND cleanup_days > 0"
        )
        .fetch_all(&*self.db_pool)
        .await?;

        for task in tasks {
            let cutoff_date = Utc::now() - chrono::Duration::days(task.cleanup_days as i64);
            
            // Find backup directories for this task
            let task_backup_dir = Path::new(backup_dir);
            if let Ok(mut entries) = fs::read_dir(task_backup_dir).await {
                while let Some(entry) = entries.next_entry().await? {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        // Check if this directory contains a backup for this task
                        let meta_file = entry_path.join("rdumper.backup.json");
                        if meta_file.exists() {
                            // Read metadata to check if it belongs to this task
                            if let Ok(meta_content) = fs::read_to_string(&meta_file).await {
                                if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&meta_content) {
                                    if let Some(task_id) = metadata.get("task_id").and_then(|v| v.as_str()) {
                                        if task_id == task.id {
                                            // Check creation date
                                            if let Some(created_at_str) = metadata.get("created_at").and_then(|v| v.as_str()) {
                                                if let Ok(created_at) = chrono::DateTime::parse_from_rfc3339(created_at_str) {
                                                    let created_at_utc = created_at.with_timezone(&Utc);
                                                    if created_at_utc < cutoff_date {
                                                        // Delete this backup directory
                                                        match fs::remove_dir_all(&entry_path).await {
                                                            Ok(_) => {
                                                                deleted_count += 1;
                                                                info!("Deleted old backup: {:?} (task: {}, age: {} days)", 
                                                                      entry_path, task.name, task.cleanup_days);
                                                            }
                                                            Err(e) => {
                                                                error!("Failed to delete backup directory {:?}: {}", entry_path, e);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }
}
