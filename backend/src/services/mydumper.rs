use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command as TokioCommand;
use tokio::fs::File;
use tracing::{error, info, warn};
use sqlx::{SqlitePool, MySqlPool, Row};

use crate::models::{DatabaseConfig, Task, CompressionType};

pub struct MydumperService {
    backup_base_dir: String,
    log_base_dir: String,
}

impl MydumperService {
    pub fn new(backup_base_dir: String, log_base_dir: String) -> Self {
        Self { backup_base_dir, log_base_dir }
    }

    /// Analyze table engines and return InnoDB tables, excluding MyISAM and other non-transactional engines
    async fn analyze_table_engines(&self, database_config: &DatabaseConfig, database_name: &str) -> Result<(Vec<String>, Vec<String>)> {
        let connection_string = database_config.connection_string_with_db(database_name);

        let pool = MySqlPool::connect(&connection_string).await?;
        
        // Query to get table names and their engines
        let query = "SELECT TABLE_NAME, ENGINE FROM information_schema.TABLES WHERE TABLE_SCHEMA = ?";
        let rows = sqlx::query(query)
            .bind(database_name)
            .fetch_all(&pool)
            .await?;

        let mut innodb_tables = Vec::new();
        let mut excluded_tables = Vec::new();

        for row in rows {
            let table_name: String = row.get("TABLE_NAME");
            let engine: String = row.get("ENGINE");
            
            match engine.to_uppercase().as_str() {
                "INNODB" => {
                    innodb_tables.push(table_name);
                }
                "MYISAM" | "MEMORY" | "CSV" | "ARCHIVE" | "FEDERATED" | "MERGE" | "BLACKHOLE" => {
                    excluded_tables.push(format!("{} ({})", table_name, engine));
                }
                _ => {
                    // For unknown engines, include them but log a warning
                    warn!("Unknown table engine '{}' for table '{}', including in backup", engine, table_name);
                    innodb_tables.push(table_name);
                }
            }
        }

        pool.close().await;

        Ok((innodb_tables, excluded_tables))
    }

    pub async fn create_backup_with_progress(
        &self,
        database_config: &DatabaseConfig,
        database_name: &str,
        task: &Task,
        job_id: String,
        pool: &SqlitePool,
    ) -> Result<String> {
        info!("Starting backup for database: {} (Job: {})", database_name, job_id);

        // Analyze table engines for logging purposes
        let (innodb_tables, excluded_tables) = self.analyze_table_engines(database_config, database_name).await?;
        
        // Log table analysis results
        info!("Database {} analysis: {} InnoDB tables, {} non-InnoDB tables will be ignored", 
              database_name, innodb_tables.len(), excluded_tables.len());
        
        if !excluded_tables.is_empty() {
            warn!("Ignoring non-InnoDB tables: {}", excluded_tables.join(", "));
            warn!("MyDumper will ignore these tables using --ignore-engines parameter");
        }

        // Create backup process using new system
        let backup_service = crate::services::FilesystemBackupService::new(self.backup_base_dir.clone());
        let mut backup_process = backup_service.create_backup_process(&job_id, database_config, Some(task)).await?;

        // Create log directory for mydumper logs
        let log_dir = format!("{}/{}", self.log_base_dir, job_id);
        std::fs::create_dir_all(&log_dir)?;

        // Create rdumper.meta.json with table information
        let table_count = (innodb_tables.len() + excluded_tables.len()) as u32;
        let meta_file = format!("{}/rdumper.meta.json", log_dir);
        
        let rdumper_meta = serde_json::json!({
            "count": table_count,
            "tables": innodb_tables.iter().map(|t| t.clone()).collect::<Vec<String>>(),
            "excluded_tables": excluded_tables.iter().map(|t| t.clone()).collect::<Vec<String>>(),
            "database_name": database_name,
            "started_at": chrono::Utc::now().to_rfc3339()
        });
        
        std::fs::write(&meta_file, serde_json::to_string_pretty(&rdumper_meta)?)?;

        info!("Database {} has {} total tables ({} InnoDB will be backed up)", 
              database_name, table_count, innodb_tables.len());

        // Create log file
        let log_file_path = format!("{}/mydumper.log", log_dir);
        let mut log_file = File::create(&log_file_path).await?;

        // Update job status to running
        self.update_job_status(pool, &job_id, "running", None, Some(&log_file_path)).await?;

        // Write initial log entry
        let start_log = format!("[{}] INFO: Starting backup for database: {}\n", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
            database_name);
        log_file.write_all(start_log.as_bytes()).await?;
        log_file.flush().await?;

        // Build mydumper command
        let mut cmd = TokioCommand::new("mydumper");
        cmd.arg("--host").arg(&database_config.host)
            .arg("--port").arg(database_config.port.to_string())
            .arg("--user").arg(&database_config.username)
            .arg("--password").arg(&database_config.password)
            .arg("--database").arg(database_name)
            .arg("--outputdir").arg(backup_process.tmp_dir())
            .arg("--verbose").arg("3")
            .arg("--threads").arg("4")
            .arg("--logfile").arg(&log_file_path)
            .arg("--triggers")
            .arg("--events")
            .arg("--routines");

        // Add non-transactional tables option if enabled
        if task.use_non_transactional {
            cmd.arg("--trx-tables").arg("0");
            cmd.arg("--no-backup-locks");
        } else {
            // For safe InnoDB-only backup, ignore non-InnoDB engines
            cmd.arg("--ignore-engines").arg("MyISAM,MEMORY,CSV,ARCHIVE,FEDERATED,MERGE,BLACKHOLE");
            info!("Ignoring non-InnoDB engines: MyISAM,MEMORY,CSV,ARCHIVE,FEDERATED,MERGE,BLACKHOLE");
        }

        // Add compression if specified
        let compression = task.compression_type().unwrap_or(CompressionType::Gzip);
        match compression {
            CompressionType::Gzip => {
                cmd.arg("--compress");
            }
            CompressionType::Zstd => {
                cmd.arg("--compress-protocol");
            }
            CompressionType::None => {
                // No compression flags
            }
        }

        info!("Executing mydumper command for database: {}", database_name);

        // Execute mydumper command and wait for completion
        let status = cmd.status().await?;

        let completion_log = format!("[{}] mydumper process completed with status: {:?}\n", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
            status.code());
        log_file.write_all(completion_log.as_bytes()).await?;
        log_file.flush().await?;

        if !status.success() {
            error!("mydumper failed with exit code: {:?}", status.code());
            let error_msg = format!("mydumper failed with exit code: {:?}", status.code());
            self.update_job_status(pool, &job_id, "failed", Some(&error_msg), Some(&log_file_path)).await?;
            return Err(anyhow!("mydumper failed: {}", error_msg));
        }

        info!("MyDumper completed successfully for database: {}", database_name);

        // Update job status to compressing before creating archive
        self.update_job_status(pool, &job_id, "compressing", None, Some(&log_file_path)).await?;

        // Complete the backup process (creates archive, calculates hash, updates metadata, cleans up tmp)
        let backup_file_path = backup_process.complete().await?;

        // Update job to completed
        self.update_job_status(pool, &job_id, "completed", None, Some(&log_file_path)).await?;

        // Update job with backup file path
        self.update_job_backup_path(pool, &job_id, &backup_file_path).await?;

        Ok(backup_file_path)
    }

    // Keep the original backup method for compatibility
    // pub async fn create_backup(
    //     &self,
    //     database_config: &DatabaseConfig,
    //     task: &Task,
    //     job_id: Option<String>,
    // ) -> Result<String> {
    //     // For backward compatibility, use a dummy job id if none provided
    //     let job_id = job_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    //     
    //     // This would need a database pool, but for now return an error
    //     Err(anyhow!("Please use create_backup_with_progress method"))
    // }

    // Helper methods for database operations

    async fn update_job_status(
        &self,
        pool: &SqlitePool,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
        log_output: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now();
        
        let mut query = "UPDATE jobs SET status = ?, updated_at = ?".to_string();
        
        if status == "running" {
            query.push_str(", started_at = ?");
        }
        
        if status == "completed" || status == "failed" || status == "cancelled" {
            query.push_str(", completed_at = ?");
        }
        
        if error_message.is_some() {
            query.push_str(", error_message = ?");
        }
        
        if log_output.is_some() {
            query.push_str(", log_output = ?");
        }
        
        query.push_str(" WHERE id = ?");
        
        let mut db_query = sqlx::query(&query)
            .bind(status)
            .bind(now);
            
        if status == "running" {
            db_query = db_query.bind(now);
        }
        
        if status == "completed" || status == "failed" || status == "cancelled" {
            db_query = db_query.bind(now);
        }
        
        if let Some(error) = error_message {
            db_query = db_query.bind(error);
        }
        
        if let Some(log) = log_output {
            db_query = db_query.bind(log);
        }
        
        db_query = db_query.bind(job_id);
        
        db_query.execute(pool).await?;
        Ok(())
    }


    async fn update_job_backup_path(&self, pool: &SqlitePool, job_id: &str, backup_path: &str) -> Result<()> {
        sqlx::query("UPDATE jobs SET backup_path = ? WHERE id = ?")
            .bind(backup_path)
            .bind(job_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // Method to parse logs and calculate real-time progress

    // Method to read logs from file
    pub async fn read_job_logs(&self, job_id: &str, pool: &SqlitePool) -> Result<String> {
        // Get job from database to find log path or backup path
        let job: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT log_output, backup_path FROM jobs WHERE id = ?"
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await?;

        if let Some((log_output, backup_path)) = job {
            // First, try to read from the log_output path if it's a file path
            if let Some(log_path) = log_output {
                // Check if it's a file path (contains .log)
                if log_path.contains(".log") && tokio::fs::metadata(&log_path).await.is_ok() {
                    let content = tokio::fs::read_to_string(&log_path).await?;
                    return Ok(content);
                }
                // If it's not a file path but contains log content, return it as is
                if !log_path.contains("/") && !log_path.contains("\\") {
                    return Ok(log_path);
                }
            }
            
            // Otherwise, try to read from log file based on backup path
            if let Some(backup_path) = backup_path {
                let base_folder = backup_path.split('/').last().unwrap_or("");
                let log_file_path = format!("{}/{}/mydumper.log", self.log_base_dir, base_folder);
                
                if tokio::fs::metadata(&log_file_path).await.is_ok() {
                    let content = tokio::fs::read_to_string(&log_file_path).await?;
                    return Ok(content);
                }
            }
        }
        
        Ok("No logs available for this job".to_string())
    }

    pub async fn restore_backup(
        &self,
        database_config: &DatabaseConfig,
        backup_path: &str,
        new_database_name: Option<&str>,
        overwrite_existing: bool,
    ) -> Result<()> {
        info!("Starting restore from backup: {}", backup_path);

        let backup_path = Path::new(backup_path);
        
        // Extract archive if it's compressed
        let source_dir = if backup_path.is_file() {
            self.extract_compressed_archive(backup_path).await?
        } else {
            backup_path.to_string_lossy().to_string()
        };

        let target_database = new_database_name.unwrap_or("restored_db");

        // If creating a new database, create it first
        if let Some(new_db_name) = new_database_name {
            info!("Creating new database: {}", new_db_name);
            self.create_database(database_config, new_db_name).await?;
        }

        // Build myloader command
        let mut cmd = TokioCommand::new("myloader");
        cmd.arg("--host").arg(&database_config.host)
            .arg("--port").arg(database_config.port.to_string())
            .arg("--user").arg(&database_config.username)
            .arg("--password").arg(&database_config.password)
            .arg("--database").arg(target_database)
            .arg("--directory").arg(&source_dir)
            .arg("--verbose").arg("3")
            .arg("--threads").arg("4");

        if overwrite_existing {
            cmd.arg("--overwrite-tables");
        }

        info!("Executing myloader command for database: {}", target_database);

        // Execute myloader command and wait for completion
        let status = cmd.status().await?;

        if !status.success() {
            error!("myloader failed with exit code: {:?}", status.code());
            return Err(anyhow!("myloader failed with exit code: {:?}", status.code()));
        }

        info!("Restore completed successfully for database: {}", target_database);

        Ok(())
    }

    async fn create_database(&self, database_config: &DatabaseConfig, database_name: &str) -> Result<()> {
        let connection_string = format!(
            "mysql://{}:{}@{}:{}/",
            database_config.username,
            database_config.password,
            database_config.host,
            database_config.port
        );

        let pool = sqlx::MySqlPool::connect(&connection_string).await?;
        
        // Create the database
        sqlx::query(&format!("CREATE DATABASE IF NOT EXISTS `{}`", database_name))
            .execute(&pool)
            .await?;
        
        info!("Database '{}' created successfully", database_name);
        Ok(())
    }

    // pub fn cleanup_old_backups(&self, database_name: &str, days: i64) -> Result<Vec<String>> {
    //     let database_backup_dir = format!("{}/{}", self.backup_base_dir, database_name);
    //     let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days);

    //     let mut deleted_backups = Vec::new();

    //     if !Path::new(&database_backup_dir).exists() {
    //         return Ok(deleted_backups);
    //     }

    //     for entry in std::fs::read_dir(&database_backup_dir)? {
    //         let entry = entry?;
    //         let path = entry.path();

    //         if path.is_dir() {
    //             if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
    //                 // Try to parse timestamp from directory name (format: YYYYMMDD_HHMMSS)
    //                 if let Ok(backup_date) = chrono::NaiveDateTime::parse_from_str(dir_name, "%Y%m%d_%H%M%S") {
    //                     let backup_date = backup_date.and_utc();
    //                     
    //                     if backup_date < cutoff_date {
    //                         info!("Deleting old backup: {}", path.display());
    //                         
    //                         if let Err(e) = std::fs::remove_dir_all(&path) {
    //                             warn!("Failed to delete backup directory {}: {}", path.display(), e);
    //                         } else {
    //                             deleted_backups.push(path.to_string_lossy().to_string());
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     Ok(deleted_backups)
    // }

    // fn calculate_directory_size(&self, dir_path: &str) -> Result<i64> {
    //     let mut total_size = 0;
    //     
    //     fn visit_dir(dir: &Path, total_size: &mut i64) -> Result<()> {
    //         for entry in std::fs::read_dir(dir)? {
    //             let entry = entry?;
    //             let path = entry.path();
    //             
    //             if path.is_dir() {
    //                 visit_dir(&path, total_size)?;
    //             } else {
    //                 *total_size += entry.metadata()?.len() as i64;
    //             }
    //         }
    //         Ok(())
    //     }
    //     
    //     visit_dir(Path::new(dir_path), &mut total_size)?;
    //     Ok(total_size)
    // }



    async fn extract_compressed_archive(&self, archive_path: &Path) -> Result<String> {
        let extract_dir = archive_path.with_extension("");
        std::fs::create_dir_all(&extract_dir)?;

        let mut cmd = TokioCommand::new("tar");
        
        if archive_path.extension().and_then(|s| s.to_str()) == Some("gz") {
            cmd.args(&["-xzf", &archive_path.to_string_lossy(), "-C", &extract_dir.to_string_lossy()]);
        } else if archive_path.extension().and_then(|s| s.to_str()) == Some("zst") {
            cmd.args(&["--zstd", "-xf", &archive_path.to_string_lossy(), "-C", &extract_dir.to_string_lossy()]);
        } else {
            return Err(anyhow!("Unsupported archive format"));
        }

        let status = cmd.status().await?;
        
        if !status.success() {
            return Err(anyhow!("Failed to extract compressed archive"));
        }

        Ok(extract_dir.to_string_lossy().to_string())
    }

    // pub fn is_mydumper_available(&self) -> bool {
    //     Command::new("mydumper")
    //         .arg("--version")
    //         .output()
    //         .map(|output| output.status.success())
    //         .unwrap_or(false)
    // }

    // pub fn is_myloader_available(&self) -> bool {
    //     Command::new("myloader")
    //         .arg("--version")
    //         .output()
    //         .map(|output| output.status.success())
    //         .unwrap_or(false)
    // }



}