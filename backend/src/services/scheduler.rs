// TaskScheduler is currently unused - keeping for future implementation
// use anyhow::Result;
// use sqlx::SqlitePool;
// use std::collections::HashMap;
// use std::sync::Arc;
// use tokio::sync::RwLock;
// use tokio_cron_scheduler::{Job, JobScheduler};
// use tracing::{error, info, warn};
// use uuid::Uuid;

// use crate::models::{Task, DatabaseConfig, Job as JobModel, CreateJobRequest, JobType};
// use crate::services::MydumperService;

// pub struct TaskScheduler {
//     scheduler: JobScheduler,
//     pool: SqlitePool,
//     mydumper_service: Arc<MydumperService>,
//     scheduled_tasks: Arc<RwLock<HashMap<String, String>>>,
// }

// impl TaskScheduler {
//     pub async fn new(pool: SqlitePool, mydumper_service: MydumperService) -> Result<Self> {
//         let scheduler = JobScheduler::new().await?;
//         let mydumper_service = Arc::new(mydumper_service);
//         
//         Ok(Self {
//             scheduler,
//             pool,
//             mydumper_service,
//             scheduled_tasks: Arc::new(RwLock::new(HashMap::new())),
//         })
//     }

//     pub async fn start(&self) -> Result<()> {
//         info!("Starting task scheduler");
//         
//         // Load and schedule existing tasks
//         self.load_and_schedule_tasks().await?;
//         
//         // Start the scheduler
//         self.scheduler.start().await?;
//         
//         info!("Task scheduler started successfully");
//         Ok(())
//     }

//     pub async fn stop(&mut self) -> Result<()> {
//         info!("Stopping task scheduler");
//         self.scheduler.shutdown().await?;
//         info!("Task scheduler stopped");
//         Ok(())
//     }

//     pub async fn schedule_task(&self, task: &Task) -> Result<()> {
//         if !task.is_active {
//             info!("Task {} is inactive, skipping scheduling", task.id);
//             return Ok(());
//         }

//         let task_id = task.id.clone();
//         let cron_expr = task.schedule.clone();
//         
//         info!("Scheduling task {} with cron expression: {}", task_id, cron_expr);
//         
//         // Create a job that will be executed
//         let job_id = Uuid::new_v4().to_string();
//         
//         // Store the mapping between job_id and task_id
//         {
//             let mut scheduled = self.scheduled_tasks.write().await;
//             scheduled.insert(job_id.clone(), task_id.clone());
//         }
//         
//         // Add the job to the scheduler
//         self.scheduler
//             .add(Job::new_async(cron_expr, move |_uuid, _l| {
//                 let task_id = task_id.clone();
//                 Box::pin(async move {
//                     info!("Executing scheduled task: {}", task_id);
//                     // Here you would execute the actual task
//                     // For now, just log that it would run
//                 })
//             })?)
//             .await?;
//         
//         info!("Task {} scheduled successfully with job ID: {}", task_id, job_id);
//         Ok(())
//     }

//     pub async fn unschedule_task(&self, task_id: &str) -> Result<()> {
//         info!("Unscheduling task: {}", task_id);
//         
//         // Find the job ID for this task
//         let job_id = {
//             let scheduled = self.scheduled_tasks.read().await;
//             scheduled.get(task_id).cloned()
//         };
//         
//         if let Some(job_id) = job_id {
//             // Remove from scheduler
//             self.scheduler.remove(&job_id).await?;
//             
//             // Remove from our tracking
//             {
//                 let mut scheduled = self.scheduled_tasks.write().await;
//                 scheduled.remove(task_id);
//             }
//             
//             info!("Task {} unscheduled successfully", task_id);
//         } else {
//             warn!("Task {} was not scheduled", task_id);
//         }
//         
//         Ok(())
//     }

//     pub async fn reschedule_task(&self, task: &Task) -> Result<()> {
//         info!("Rescheduling task: {}", task.id);
//         
//         // First unschedule the existing task
//         self.unschedule_task(&task.id).await?;
//         
//         // Then schedule it again with new parameters
//         self.schedule_task(task).await?;
//         
//         info!("Task {} rescheduled successfully", task.id);
//         Ok(())
//     }

//     pub async fn run_task_immediately(&self, task_id: &str) -> Result<String> {
//         info!("Running task immediately: {}", task_id);
//         
//         // Get task from database
//         let task: Task = sqlx::query_as(
//             "SELECT * FROM tasks WHERE id = ?"
//         )
//         .bind(task_id)
//         .fetch_optional(&self.pool)
//         .await?
//         .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;
//         
//         // Get database config
//         let db_config: DatabaseConfig = sqlx::query_as(
//             "SELECT * FROM database_configs WHERE id = ?"
//         )
//         .bind(&task.database_config_id)
//         .fetch_optional(&self.pool)
//         .await?
//         .ok_or_else(|| anyhow::anyhow!("Database config not found: {}", task.database_config_id))?;

//         // Create job record
//         let job_request = CreateJobRequest {
//             task_id: Some(task_id.to_string()),
//             job_type: JobType::Backup,
//             backup_path: None,
//         };

//         let job = JobModel::new(job_request);

//         // Insert job into database
//         sqlx::query(
//             r#"
//             INSERT INTO jobs (id, task_id, job_type, status, progress, started_at, completed_at, error_message, log_output, backup_path, created_at)
//             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
//             "#
//         )
//         .bind(&job.id)
//         .bind(&job.task_id)
//         .bind(&job.job_type)
//         .bind(&job.status)
//         .bind(&job.progress)
//         .bind(&job.started_at)
//         .bind(&job.completed_at)
//         .bind(&job.error_message)
//         .bind(&job.log_output)
//         .bind(&job.backup_path)
//         .bind(&job.created_at)
//         .execute(&self.pool)
//         .await?;

//         // Execute the task
//         let result = self.mydumper_service
//             .create_backup_with_progress(&db_config, &task, job.id.clone(), &self.pool)
//             .await;

//         match result {
//             Ok(backup_path) => {
//                 info!("Task {} completed successfully", task_id);
//                 Ok(job.id)
//             }
//             Err(e) => {
//                 error!("Task {} failed: {}", task_id, e);
//                 Err(e)
//             }
//         }
//     }

//     async fn load_and_schedule_tasks(&self) -> Result<()> {
//         info!("Loading and scheduling existing tasks");
//         
//         let tasks: Vec<Task> = sqlx::query_as(
//             "SELECT * FROM tasks WHERE is_active = true"
//         )
//         .fetch_all(&self.pool)
//         .await?;
//         
//         for task in tasks {
//             if let Err(e) = self.schedule_task(&task).await {
//                 error!("Failed to schedule task {}: {}", task.id, e);
//             }
//         }
//         
//         info!("Loaded and scheduled {} tasks", tasks.len());
//         Ok(())
//     }

//     pub async fn schedule_cleanup_jobs(&self) -> Result<()> {
//         info!("Scheduling cleanup jobs");
//         
//         // Schedule daily cleanup at 2 AM
//         self.scheduler
//             .add(Job::new_async("0 0 2 * * *", |_uuid, _l| {
//                 Box::pin(async move {
//                     info!("Running scheduled cleanup");
//                     // Here you would run cleanup tasks
//                 })
//             })?)
//             .await?;
//         
//         info!("Cleanup jobs scheduled successfully");
//         Ok(())
//     }
// }