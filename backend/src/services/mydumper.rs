use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt};
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
    async fn analyze_table_engines(&self, database_config: &DatabaseConfig) -> Result<(Vec<String>, Vec<String>)> {
        let connection_string = format!(
            "mysql://{}:{}@{}:{}/{}",
            database_config.username,
            database_config.password,
            database_config.host,
            database_config.port,
            database_config.database_name
        );

        let pool = MySqlPool::connect(&connection_string).await?;
        
        // Query to get table names and their engines
        let query = "SELECT TABLE_NAME, ENGINE FROM information_schema.TABLES WHERE TABLE_SCHEMA = ?";
        let rows = sqlx::query(query)
            .bind(&database_config.database_name)
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
        task: &Task,
        job_id: String,
        pool: &SqlitePool,
    ) -> Result<String> {
        info!("Starting backup for database: {} (Job: {})", database_config.database_name, job_id);

        // Analyze table engines for logging purposes
        let (innodb_tables, excluded_tables) = self.analyze_table_engines(database_config).await?;
        
        // Log table analysis results
        info!("Database {} analysis: {} InnoDB tables, {} non-InnoDB tables will be ignored", 
              database_config.database_name, innodb_tables.len(), excluded_tables.len());
        
        if !excluded_tables.is_empty() {
            warn!("Ignoring non-InnoDB tables: {}", excluded_tables.join(", "));
            warn!("MyDumper will ignore these tables using --ignore-engines parameter");
        }

        // Create directory structure: <database>-<datum>/temp and logs
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let base_folder = format!("{}-{}", database_config.database_name, timestamp);
        
        let backup_dir = format!("{}/{}/temp", self.backup_base_dir, base_folder);
        let log_dir = format!("{}/{}", self.log_base_dir, base_folder);

        // Create directories
        std::fs::create_dir_all(&backup_dir)?;
        std::fs::create_dir_all(&log_dir)?;

        // Use total table count for progress tracking (MyDumper will handle filtering)
        let table_count = (innodb_tables.len() + excluded_tables.len()) as u32;
        let count_file = format!("{}/count_tables.txt", log_dir);
        std::fs::write(&count_file, table_count.to_string())?;

        info!("Database {} has {} total tables ({} InnoDB will be backed up)", 
              database_config.database_name, table_count, innodb_tables.len());

        // Create log file
        let log_file_path = format!("{}/mydumper.log", log_dir);
        let mut log_file = File::create(&log_file_path).await?;

        // Update job status to running
        self.update_job_status(pool, &job_id, "running", None, None).await?;

        // First, run mydumper --help to show all available options
        info!("Running mydumper --help to show available options");
        let help_log = format!("[{}] INFO: Running mydumper --help to show available options\n", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
        log_file.write_all(help_log.as_bytes()).await?;
        log_file.flush().await?;

        let help_cmd = TokioCommand::new("mydumper")
            .arg("--help")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let help_output = help_cmd.wait_with_output().await?;
        
        // Log the help output
        let help_stdout = String::from_utf8_lossy(&help_output.stdout);
        let help_stderr = String::from_utf8_lossy(&help_output.stderr);
        
        let help_log_entry = format!("[{}] INFO: MyDumper Help Output:\n{}\n{}\n", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
            help_stdout, 
            help_stderr);
        log_file.write_all(help_log_entry.as_bytes()).await?;
        log_file.flush().await?;

        // Build mydumper command
        let mut cmd = TokioCommand::new("mydumper");
        cmd.arg("--host").arg(&database_config.host)
            .arg("--port").arg(database_config.port.to_string())
            .arg("--user").arg(&database_config.username)
            .arg("--password").arg(&database_config.password)
            .arg("--database").arg(&database_config.database_name)
            .arg("--outputdir").arg(&backup_dir)
            .arg("--verbose").arg("3")
            .arg("--threads").arg("4")
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

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!("Executing mydumper command for database: {}", database_config.database_name);

        // Write initial log entry
        let start_log = format!("[{}] Starting backup for database: {}\n", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
            database_config.database_name);
        log_file.write_all(start_log.as_bytes()).await?;
        log_file.flush().await?;

        let mut child = cmd.spawn()?;

        // Read stdout and stderr asynchronously
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();


        // Monitor the process output
        loop {
            tokio::select! {
                stdout_line = stdout_reader.next_line() => {
                    match stdout_line? {
                        Some(line) => {
                            info!("mydumper stdout: {}", line);
                            
                            // Write to log file
                            let log_entry = format!("[{}] {}\n", 
                                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                                line);
                            log_file.write_all(log_entry.as_bytes()).await?;
                            log_file.flush().await?;
                            
                            // Parse progress from mydumper output
                            if let Some(progress) = self.parse_mydumper_progress(&line, table_count) {
                                // Update job progress
                                self.update_job_progress(pool, &job_id, progress).await?;
                            }
                        }
                        None => break,
                    }
                }
                stderr_line = stderr_reader.next_line() => {
                    match stderr_line? {
                        Some(line) => {
                            // Parse stderr content to determine log level
                            if line.starts_with("** Message:") {
                                // These are normal informational messages from mydumper
                                info!("mydumper: {}", line);
                                
                                let log_entry = format!("[{}] INFO: {}\n", 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                                    line);
                                log_file.write_all(log_entry.as_bytes()).await?;
                                log_file.flush().await?;
                            } else if line.to_lowercase().contains("error") || 
                                     line.to_lowercase().contains("failed") || 
                                     line.to_lowercase().contains("fatal") {
                                // These are actual errors
                                error!("mydumper error: {}", line);
                                
                                let log_entry = format!("[{}] ERROR: {}\n", 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                                    line);
                                log_file.write_all(log_entry.as_bytes()).await?;
                                log_file.flush().await?;
                            } else {
                                // Other stderr output (warnings, info, etc.)
                                info!("mydumper: {}", line);
                                
                                let log_entry = format!("[{}] INFO: {}\n", 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                                    line);
                                log_file.write_all(log_entry.as_bytes()).await?;
                                log_file.flush().await?;
                                
                                // Also parse progress from stderr
                                if let Some(progress) = self.parse_mydumper_progress(&line, table_count) {
                                    // Update job progress
                                    self.update_job_progress(pool, &job_id, progress).await?;
                                }
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        // Wait for the process to complete
        let status = child.wait().await?;

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

        info!("Backup completed successfully for database: {}", database_config.database_name);

        // Update job to completed
        self.update_job_status(pool, &job_id, "completed", None, Some(&log_file_path)).await?;
        self.update_job_progress(pool, &job_id, 100).await?;

        // Compress the backup and clean up temp directory
        let final_backup_path = self.compress_and_cleanup_backup(&backup_dir, &base_folder, &compression).await?;
        self.update_job_backup_path(pool, &job_id, &final_backup_path).await?;

        Ok(final_backup_path)
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

    async fn update_job_progress(&self, pool: &SqlitePool, job_id: &str, progress: i32) -> Result<()> {
        sqlx::query("UPDATE jobs SET progress = ? WHERE id = ?")
            .bind(progress)
            .bind(job_id)
            .execute(pool)
            .await?;
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
    pub async fn get_job_progress_from_logs(&self, _job_id: &str) -> Result<(i32, String)> {
        // Try to find the log directory for this job
        let _log_pattern = format!("{}/*/mydumper.log", self.log_base_dir);
        
        // This is a simplified version - in practice, you'd want to store the log path in the job
        // For now, let's return a placeholder
        Ok((0, "No logs found".to_string()))
    }

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

        let target_database = new_database_name.unwrap_or(&database_config.database_name);

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

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!("Executing myloader command for database: {}", target_database);

        let mut child = cmd.spawn()?;

        // Read output similar to backup process
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut output_log = String::new();

        loop {
            tokio::select! {
                stdout_line = stdout_reader.next_line() => {
                    match stdout_line? {
                        Some(line) => {
                            info!("myloader stdout: {}", line);
                            output_log.push_str(&format!("{}\n", line));
                        }
                        None => break,
                    }
                }
                stderr_line = stderr_reader.next_line() => {
                    match stderr_line? {
                        Some(line) => {
                            // Parse stderr content to determine log level
                            if line.starts_with("** Message:") {
                                // These are normal informational messages from myloader
                                info!("myloader: {}", line);
                                output_log.push_str(&format!("INFO: {}\n", line));
                            } else if line.to_lowercase().contains("error") || 
                                     line.to_lowercase().contains("failed") || 
                                     line.to_lowercase().contains("fatal") {
                                // These are actual errors
                                error!("myloader error: {}", line);
                                output_log.push_str(&format!("ERROR: {}\n", line));
                            } else {
                                // Other stderr output (warnings, info, etc.)
                                info!("myloader: {}", line);
                                output_log.push_str(&format!("INFO: {}\n", line));
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        let status = child.wait().await?;

        if !status.success() {
            error!("myloader failed with exit code: {:?}", status.code());
            return Err(anyhow!("myloader failed: {}", output_log));
        }

        info!("Restore completed successfully for database: {}", target_database);

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

    async fn create_compressed_archive(&self, source_dir: &str, compression: &CompressionType) -> Result<String> {
        let archive_path = match compression {
            CompressionType::Gzip => format!("{}.tar.gz", source_dir),
            CompressionType::Zstd => format!("{}.tar.zst", source_dir),
            CompressionType::None => return Ok(source_dir.to_string()),
        };

        let mut cmd = TokioCommand::new("tar");
        
        match compression {
            CompressionType::Gzip => {
                cmd.args(&["-czf", &archive_path, "-C", source_dir, "."]);
            }
            CompressionType::Zstd => {
                cmd.args(&["--zstd", "-cf", &archive_path, "-C", source_dir, "."]);
            }
            CompressionType::None => unreachable!(),
        }

        let status = cmd.status().await?;
        
        if !status.success() {
            return Err(anyhow!("Failed to create compressed archive"));
        }

        // Remove the original directory after successful compression
        if let Err(e) = std::fs::remove_dir_all(source_dir) {
            warn!("Failed to remove source directory after compression: {}", e);
        }

        Ok(archive_path)
    }

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

    async fn compress_and_cleanup_backup(
        &self,
        temp_backup_dir: &str,
        base_folder: &str,
        compression: &CompressionType,
    ) -> Result<String> {
        info!("Compressing backup and cleaning up temp directory");

        // Create the final backup directory (without temp)
        let final_backup_dir = format!("{}/{}", self.backup_base_dir, base_folder);
        std::fs::create_dir_all(&final_backup_dir)?;

        // Move all files from temp to final directory
        if let Ok(entries) = std::fs::read_dir(temp_backup_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let src_path = entry.path();
                    let dst_path = std::path::Path::new(&final_backup_dir).join(entry.file_name());
                    
                    if let Err(e) = std::fs::rename(&src_path, &dst_path) {
                        warn!("Failed to move file {:?} to {:?}: {}", src_path, dst_path, e);
                    }
                }
            }
        }

        // Remove the temp directory
        if let Err(e) = std::fs::remove_dir(temp_backup_dir) {
            warn!("Failed to remove temp directory {}: {}", temp_backup_dir, e);
        }

        // Create compressed archive
        let archive_path = self.create_compressed_archive(&final_backup_dir, compression).await?;

        // Remove the uncompressed directory after successful compression
        if let Err(e) = std::fs::remove_dir_all(&final_backup_dir) {
            warn!("Failed to remove uncompressed directory after compression: {}", e);
        }

        info!("Backup compression and cleanup completed: {}", archive_path);
        Ok(archive_path)
    }

    /// Parse mydumper progress from log lines
    /// Looks for patterns like: `Thread X: \`database\`.\`table\` [ 25% ] | Tables: 5/24`
    fn parse_mydumper_progress(&self, line: &str, total_tables: u32) -> Option<i32> {
        // Look for progress pattern: [ XX% ] | Tables: X/Y
        if let Some(percent_start) = line.find("[ ") {
            if let Some(percent_end) = line[percent_start + 2..].find("% ]") {
                let percent_str = &line[percent_start + 2..percent_start + 2 + percent_end];
                if let Ok(percent) = percent_str.trim().parse::<i32>() {
                    // Cap at 100% to avoid overflows
                    return Some(percent.min(100));
                }
            }
        }
        
        // Alternative: Look for "Tables: X/Y" pattern to calculate progress
        if let Some(tables_start) = line.find("Tables: ") {
            if let Some(tables_end) = line[tables_start + 8..].find("/") {
                let current_tables_str = &line[tables_start + 8..tables_start + 8 + tables_end];
                if let Ok(current_tables) = current_tables_str.parse::<u32>() {
                    if total_tables > 0 {
                        let progress = ((total_tables - current_tables) as f64 / total_tables as f64 * 100.0) as i32;
                        return Some(progress.min(100));
                    }
                }
            }
        }
        
        None
    }
}