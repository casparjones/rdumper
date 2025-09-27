use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler, JobToRun};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::{Task, DatabaseConfig, Job as JobModel, CreateJobRequest, JobType, JobStatus};
use crate::services::MydumperService;

pub struct TaskScheduler {
    scheduler: JobScheduler,
    pool: SqlitePool,
    mydumper_service: Arc<MydumperService>,
    scheduled_jobs: Arc<RwLock<HashMap<String, Uuid>>>, // task_id -> scheduler_job_id
}

impl TaskScheduler {
    pub async fn new(pool: SqlitePool, mydumper_service: MydumperService) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        
        Ok(Self {
            scheduler,
            pool,
            mydumper_service: Arc::new(mydumper_service),
            scheduled_jobs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting task scheduler");
        
        // Load existing active tasks and schedule them
        self.load_and_schedule_tasks().await?;
        
        // Start the scheduler
        self.scheduler.start().await?;
        
        info!("Task scheduler started successfully");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping task scheduler");
        self.scheduler.shutdown().await?;
        info!("Task scheduler stopped");
        Ok(())
    }

    pub async fn schedule_task(&self, task: &Task) -> Result<()> {
        if !task.is_active {
            info!("Task {} is inactive, skipping scheduling", task.name);
            return Ok(());
        }

        info!("Scheduling task: {} with cron: {}", task.name, task.cron_schedule);

        let task_id = task.id.clone();
        let pool = self.pool.clone();
        let mydumper_service = self.mydumper_service.clone();
        let scheduled_jobs = self.scheduled_jobs.clone();

        let job = Job::new_async(task.cron_schedule.as_str(), move |_uuid, _l| {
            let task_id = task_id.clone();
            let pool = pool.clone();
            let mydumper_service = mydumper_service.clone();
            let _scheduled_jobs = scheduled_jobs.clone();

            Box::pin(async move {
                if let Err(e) = execute_backup_task(task_id, pool, mydumper_service).await {
                    error!("Failed to execute scheduled backup task: {}", e);
                }
            })
        })?;

        let scheduler_job_id = self.scheduler.add(job).await?;
        
        // Store the mapping
        {
            let mut jobs = self.scheduled_jobs.write().await;
            jobs.insert(task.id.clone(), scheduler_job_id);
        }

        info!("Task {} scheduled successfully", task.name);
        Ok(())
    }

    pub async fn unschedule_task(&self, task_id: &str) -> Result<()> {
        let mut jobs = self.scheduled_jobs.write().await;
        
        if let Some(scheduler_job_id) = jobs.remove(task_id) {
            self.scheduler.remove(&scheduler_job_id).await?;
            info!("Task {} unscheduled successfully", task_id);
        } else {
            warn!("Task {} was not found in scheduled jobs", task_id);
        }
        
        Ok(())
    }

    pub async fn reschedule_task(&self, task: &Task) -> Result<()> {
        // Remove existing schedule
        self.unschedule_task(&task.id).await?;
        
        // Add new schedule if task is active
        if task.is_active {
            self.schedule_task(task).await?;
        }
        
        Ok(())
    }

    pub async fn run_task_immediately(&self, task_id: &str) -> Result<String> {
        info!("Running task {} immediately", task_id);

        // Create a job for immediate execution
        let job_request = CreateJobRequest {
            task_id: Some(task_id.to_string()),
            job_type: JobType::Backup,
            backup_path: None,
        };

        let job = JobModel::new(job_request);
        let job_id = job.id.clone();

        // Insert job into database
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
        .execute(&self.pool)
        .await?;

        // Execute the task asynchronously
        let task_id = task_id.to_string();
        let pool = self.pool.clone();
        let mydumper_service = self.mydumper_service.clone();

        tokio::spawn(async move {
            if let Err(e) = execute_backup_task(task_id, pool, mydumper_service).await {
                error!("Failed to execute immediate backup task: {}", e);
            }
        });

        Ok(job_id)
    }

    async fn load_and_schedule_tasks(&self) -> Result<()> {
        info!("Loading existing tasks for scheduling");

        let tasks: Vec<Task> = sqlx::query_as(
            "SELECT * FROM tasks WHERE is_active = true"
        )
        .fetch_all(&self.pool)
        .await?;

        for task in tasks {
            if let Err(e) = self.schedule_task(&task).await {
                error!("Failed to schedule task {}: {}", task.name, e);
            }
        }

        info!("Loaded and scheduled {} tasks", self.scheduled_jobs.read().await.len());
        Ok(())
    }

    pub async fn schedule_cleanup_jobs(&self) -> Result<()> {
        info!("Scheduling cleanup jobs");

        let pool = self.pool.clone();
        let mydumper_service = self.mydumper_service.clone();

        // Schedule daily cleanup at 2 AM
        let cleanup_job = Job::new_async("0 2 * * *", move |_uuid, _l| {
            let pool = pool.clone();
            let mydumper_service = mydumper_service.clone();

            Box::pin(async move {
                if let Err(e) = execute_cleanup_tasks(pool, mydumper_service).await {
                    error!("Failed to execute cleanup tasks: {}", e);
                }
            })
        })?;

        self.scheduler.add(cleanup_job).await?;
        info!("Cleanup jobs scheduled successfully");
        
        Ok(())
    }
}

async fn execute_backup_task(
    task_id: String,
    pool: SqlitePool,
    mydumper_service: Arc<MydumperService>,
) -> Result<()> {
    info!("Executing backup task: {}", task_id);

    // Get task details
    let task: Task = sqlx::query_as(
        "SELECT * FROM tasks WHERE id = ? AND is_active = true"
    )
    .bind(&task_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("Task not found or inactive: {}", task_id))?;

    // Get database config
    let db_config: DatabaseConfig = sqlx::query_as(
        "SELECT * FROM database_configs WHERE id = ?"
    )
    .bind(&task.database_config_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("Database config not found: {}", task.database_config_id))?;

    // Create job record
    let job_request = CreateJobRequest {
        task_id: Some(task_id.clone()),
        job_type: JobType::Backup,
        backup_path: None,
    };

    let mut job = JobModel::new(job_request);
    job.start();

    // Insert job into database
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

    // Execute backup
    match mydumper_service.create_backup(&db_config, &task, Some(job.id.clone())).await {
        Ok(backup_path) => {
            // Create backup record
            let backup_request = crate::models::CreateBackupRequest {
                database_config_id: db_config.id.clone(),
                task_id: Some(task.id.clone()),
                file_path: backup_path.clone(),
                file_size: std::fs::metadata(&backup_path)
                    .map(|m| m.len() as i64)
                    .unwrap_or(0),
                compression_type: task.compression_type.clone(),
            };

            let backup = crate::models::Backup::new(backup_request);

            // Insert backup record
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

            // Update job as completed
            job.complete();
            job.backup_path = Some(backup_path);

            sqlx::query(
                "UPDATE jobs SET status = ?, completed_at = ?, progress = ?, backup_path = ? WHERE id = ?"
            )
            .bind(&job.status)
            .bind(&job.completed_at)
            .bind(&job.progress)
            .bind(&job.backup_path)
            .bind(&job.id)
            .execute(&pool)
            .await?;

            info!("Backup task {} completed successfully", task_id);

            // Cleanup old backups for this task
            if task.cleanup_days > 0 {
                if let Err(e) = mydumper_service.cleanup_old_backups(&db_config.database_name, task.cleanup_days as i64) {
                    error!("Failed to cleanup old backups: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Backup task {} failed: {}", task_id, e);
            
            job.fail(e.to_string());

            sqlx::query(
                "UPDATE jobs SET status = ?, completed_at = ?, error_message = ? WHERE id = ?"
            )
            .bind(&job.status)
            .bind(&job.completed_at)
            .bind(&job.error_message)
            .bind(&job.id)
            .execute(&pool)
            .await?;
        }
    }

    Ok(())
}

async fn execute_cleanup_tasks(
    pool: SqlitePool,
    mydumper_service: Arc<MydumperService>,
) -> Result<()> {
    info!("Executing cleanup tasks");

    // Get all tasks with cleanup enabled
    let tasks: Vec<Task> = sqlx::query_as(
        "SELECT * FROM tasks WHERE cleanup_days > 0"
    )
    .fetch_all(&pool)
    .await?;

    for task in tasks {
        // Get database config
        if let Ok(Some(db_config)) = sqlx::query_as::<_, DatabaseConfig>(
            "SELECT * FROM database_configs WHERE id = ?"
        )
        .bind(&task.database_config_id)
        .fetch_optional(&pool)
        .await
        {
            if let Err(e) = mydumper_service.cleanup_old_backups(
                &db_config.database_name,
                task.cleanup_days as i64,
            ) {
                error!("Failed to cleanup old backups for {}: {}", db_config.database_name, e);
            }
        }
    }

    info!("Cleanup tasks completed");
    Ok(())
}