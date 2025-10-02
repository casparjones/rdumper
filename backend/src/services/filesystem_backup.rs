use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{warn, info};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{Backup, BackupMetadata, DatabaseConfigInfo, TaskInfo, DatabaseConfig, Task};
use crate::services::backup_process::BackupProcess;

pub struct FilesystemBackupService {
    backup_base_dir: String,
}

impl FilesystemBackupService {
    pub fn new(backup_base_dir: String) -> Self {
        Self { backup_base_dir }
    }
    
    /// Generate a human-readable backup directory name: <db-name>-<uuid>
    fn generate_backup_directory_name(&self, database_config: &DatabaseConfig, task: Option<&Task>) -> String {
        let uuid = uuid::Uuid::new_v4().to_string();
        
        // Use used_database format if we have task info, otherwise fall back to config name
        let db_name = if let Some(task) = task {
            let database_name = match &task.database_name {
                Some(db_name) => db_name.clone(),
                None => database_config.database_name.clone(),
            };
            format!("{}-{}", database_config.name, database_name)
        } else {
            database_config.name.clone()
        };
        
        // Sanitize database name to be filesystem-safe
        let sanitized_name = db_name
            .replace(" ", "_")
            .replace("/", "_")
            .replace("\\", "_")
            .replace(":", "_")
            .replace("*", "_")
            .replace("?", "_")
            .replace("\"", "_")
            .replace("<", "_")
            .replace(">", "_")
            .replace("|", "_");
        format!("{}-{}", sanitized_name, uuid)
    }
    
    /// Create a new backup process
    pub async fn create_backup_process(
        &self,
        backup_id: &str,
        database_config: &DatabaseConfig,
        task: Option<&Task>,
    ) -> Result<BackupProcess> {
        // Use the human-readable directory name instead of just the backup_id
        let directory_name = self.generate_backup_directory_name(database_config, task);
        let root_dir = Path::new(&self.backup_base_dir).join(&directory_name);
        let compression_type = task.map(|t| t.compression_type.clone()).unwrap_or_else(|| "gzip".to_string());
        let backup_type = "scheduled".to_string();
        
        let backup_process = BackupProcess::new(
            backup_id.to_string(),
            root_dir,
            database_config.clone(),
            task.cloned(),
            backup_type,
            compression_type,
        );
        
        // Initialize the backup process
        backup_process.initialize().await?;
        
        Ok(backup_process)
    }

    /// Scan filesystem for all backups and return them as Backup structs
    pub async fn scan_backups(&self) -> Result<Vec<Backup>> {
        let mut backups = Vec::new();
        
        if !Path::new(&self.backup_base_dir).exists() {
            return Ok(backups);
        }

        // Recursively search for backup files
        self.scan_directory_recursive(Path::new(&self.backup_base_dir), &mut backups).await?;

        // Sort by creation date (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(backups)
    }

    /// Recursively scan directory for backup files
    async fn scan_directory_recursive(&self, dir_path: &Path, backups: &mut Vec<Backup>) -> Result<()> {
        tracing::info!("Scanning directory: {:?}", dir_path);
        let mut entries = fs::read_dir(dir_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                tracing::info!("Found directory: {:?}", path);
                // Check if this is a backup folder (contains rdumper.backup.json)
                let meta_file = path.join("rdumper.backup.json");
                tracing::info!("Checking for metadata file: {:?}", meta_file);
                if meta_file.exists() {
                    tracing::info!("Found metadata file, processing backup folder");
                    // This is a backup folder, load its metadata
                    match self.load_backup_metadata(&meta_file).await {
                        Ok(metadata) => {
                            // Find the backup file in this folder
                            if let Some(backup_file) = self.find_backup_file_in_folder(&path).await? {
                                let backup = Backup {
                                    id: metadata.id,
                                    database_name: metadata.database_name,
                                    database_config_id: metadata.database_config_id,
                                    task_id: metadata.task_id,
                                    used_database: metadata.used_database,
                                    file_path: backup_file.to_string_lossy().to_string(),
                                    meta_path: meta_file.to_string_lossy().to_string(),
                                    file_size: metadata.file_size,
                                    compression_type: metadata.compression_type,
                                    created_at: metadata.created_at,
                                    backup_type: metadata.backup_type,
                                };
                                backups.push(backup);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to load metadata from {}: {}", meta_file.display(), e);
                        }
                    }
                } else {
                    // Check if this directory contains backup files without metadata
                    if let Some(backup_file) = self.find_backup_file_in_folder(&path).await? {
                        // Found a backup file without metadata, create it
                        info!("Found backup file without metadata: {}, creating metadata", backup_file.display());
                        let meta_path = self.create_metadata_file_for_backup(&backup_file).await?;
                        let backup = self.create_dummy_backup(&backup_file, &meta_path).await?;
                        backups.push(backup);
                    } else {
                        // Recursively scan subdirectories that are not backup folders
                        Box::pin(self.scan_directory_recursive(&path, backups)).await?;
                    }
                }
            } else if path.is_file() {
                // Check if this is a backup file in the root directory
                if self.is_backup_file(&path).is_some() {
                    // Found a backup file without metadata, create it
                    info!("Found backup file without metadata: {}, creating metadata", path.display());
                    let meta_path = self.create_metadata_file_for_backup(&path).await?;
                    let backup = self.create_dummy_backup(&path, &meta_path).await?;
                    backups.push(backup);
                }
            }
        }
        
        Ok(())
    }

    /// Find backup file in a folder
    async fn find_backup_file_in_folder(&self, folder_path: &Path) -> Result<Option<PathBuf>> {
        let mut entries = fs::read_dir(folder_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(".tar.gz") || file_name.ends_with(".tar.zst") || file_name.ends_with(".tar") {
                        return Ok(Some(path));
                    }
                }
            }
        }
        
        Ok(None)
    }

    /// Check if a file is a backup file
    fn is_backup_file(&self, path: &Path) -> Option<String> {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.ends_with(".tar.gz") || file_name.ends_with(".tar.zst") || file_name.ends_with(".tar") {
                return Some(file_name.to_string());
            }
        }
        None
    }

    /// Find metadata file for a backup file
    async fn find_metadata_file(&self, backup_path: &Path) -> Result<Option<PathBuf>> {
        let backup_dir = backup_path.parent().ok_or_else(|| anyhow!("No parent directory"))?;
        let backup_name = backup_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid backup filename"))?;
        
        // Look for rdumper.backup.json in the same directory
        let meta_file = backup_dir.join("rdumper.backup.json");
        if meta_file.exists() {
            return Ok(Some(meta_file));
        }
        
        // Look for metadata file with same name as backup
        let meta_file = backup_dir.join(format!("{}.backup.json", backup_name));
        if meta_file.exists() {
            return Ok(Some(meta_file));
        }
        
        Ok(None)
    }

    /// Create metadata file for a backup
    async fn create_metadata_file_for_backup(&self, backup_path: &Path) -> Result<PathBuf> {
        let backup_dir = backup_path.parent().ok_or_else(|| anyhow!("No parent directory"))?;
        let meta_file = backup_dir.join("rdumper.backup.json");
        
        // Get file metadata
        let file_metadata = fs::metadata(backup_path).await?;
        let file_size = file_metadata.len() as i64;
        let modified_time = file_metadata.modified()?;
        let modified_timestamp = modified_time.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        
        // Determine compression type from file extension
        let compression_type = if backup_path.to_string_lossy().ends_with(".tar.zst") {
            "zstd"
        } else if backup_path.to_string_lossy().ends_with(".tar.gz") {
            "gzip"
        } else if backup_path.to_string_lossy().ends_with(".tar") {
            "none"
        } else {
            "unknown"
        };
        
        // Extract information from filename
        let filename = backup_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        // Try to extract database name and timestamp from filename
        // Expected format: <database>-<timestamp>.tar.gz
        let (database_name, created_at, ident) = self.parse_backup_filename(filename, file_size, modified_timestamp);
        
        // Create metadata with extracted information
        let dummy_metadata = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "database_name": database_name,
            "database_config_id": "unknown",
            "task_id": null,
            "used_database": null,
            "file_path": backup_path.to_string_lossy(),
            "meta_path": meta_file.to_string_lossy(),
            "file_size": file_size,
            "compression_type": compression_type,
            "created_at": created_at,
            "backup_type": "external",
            "ident": ident,
            "database_config": {
                "id": "unknown",
                "name": format!("Unknown Database ({})", database_name),
                "host": "unknown",
                "port": 3306,
                "username": "unknown",
                "database_name": database_name
            },
            "task_info": null
        });
        
        fs::write(&meta_file, serde_json::to_string_pretty(&dummy_metadata)?).await?;
        Ok(meta_file)
    }

    /// Parse backup filename to extract database name, timestamp, and create ident
    fn parse_backup_filename(&self, filename: &str, file_size: i64, modified_timestamp: u64) -> (String, String, String) {
        // Remove file extension
        let name_without_ext = filename
            .replace(".tar.gz", "")
            .replace(".tar.zst", "")
            .replace(".tar", "");
        
        // Try to parse format: <database>-<timestamp>
        // Expected format: sbtest-20250929_131944
        if let Some(dash_pos) = name_without_ext.rfind('-') {
            let database_name = name_without_ext[..dash_pos].to_string();
            let timestamp_part = &name_without_ext[dash_pos + 1..];
            
            // Try to parse timestamp format: YYYYMMDD_HHMMSS
            if timestamp_part.len() == 15 && timestamp_part.chars().all(|c| c.is_ascii_digit() || c == '_') {
                // Parse the timestamp
                if let Ok(parsed_time) = chrono::NaiveDateTime::parse_from_str(timestamp_part, "%Y%m%d_%H%M%S") {
                    let created_at = parsed_time.and_utc().to_rfc3339();
                    let ident = format!("size_{}_modified_{}", file_size, modified_timestamp);
                    return (database_name, created_at, ident);
                }
            }
        }
        
        // Fallback: use filename as database name and current time
        let database_name = name_without_ext.to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let ident = format!("size_{}_modified_{}", file_size, modified_timestamp);
        
        (database_name, created_at, ident)
    }

    /// Create dummy backup entry
    async fn create_dummy_backup(&self, backup_path: &Path, meta_path: &Path) -> Result<Backup> {
        // Load the metadata we just created to get the extracted information
        let metadata = self.load_backup_metadata(meta_path).await?;
        
        let backup = Backup {
            id: metadata.id,
            database_name: metadata.database_name,
            database_config_id: metadata.database_config_id,
            task_id: metadata.task_id,
            used_database: metadata.used_database,
            file_path: backup_path.to_string_lossy().to_string(),
            meta_path: meta_path.to_string_lossy().to_string(),
            file_size: metadata.file_size,
            compression_type: metadata.compression_type,
            created_at: metadata.created_at,
            backup_type: metadata.backup_type,
        };
        
        Ok(backup)
    }

    /// Load backup metadata from JSON file
    pub async fn load_backup_metadata(&self, meta_path: &Path) -> Result<BackupMetadata> {
        let content = fs::read_to_string(meta_path).await?;
        let metadata: BackupMetadata = serde_json::from_str(&content)?;
        Ok(metadata)
    }

    /// Save backup metadata to JSON file
    pub async fn save_backup_metadata(&self, metadata: &BackupMetadata) -> Result<()> {
        let content = serde_json::to_string_pretty(metadata)?;
        fs::write(&metadata.meta_path, content).await?;
        Ok(())
    }

    /// Create a tar.gz file from a directory
    async fn create_tar_gz_from_directory(&self, source_dir: &str, output_path: &Path) -> Result<()> {
        use tokio::process::Command;
        
        // Wait a moment to ensure all files are written
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        
        let mut cmd = Command::new("tar");
        cmd.args(&[
            "-czf", 
            output_path.to_str().unwrap(), 
            "-C", 
            source_dir, 
            "--warning=no-file-changed",
            "."
        ]);
        
        let status = cmd.status().await?;
        
        if !status.success() {
            return Err(anyhow!("Failed to create tar.gz archive"));
        }
        
        // Remove the original mydumper files after creating the archive
        if let Ok(entries) = std::fs::read_dir(source_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.file_name() != Some(std::ffi::OsStr::new("backup.tar.gz")) {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Create a new backup entry from an existing directory (for mydumper output)
    pub async fn create_backup_from_directory(
        &self,
        database_config: &DatabaseConfig,
        task: Option<&Task>,
        backup_type: &str,
        source_dir: &str, // Directory containing mydumper output files (tmp/)
        compression_type: &str,
    ) -> Result<Backup> {
        let source_path = Path::new(source_dir);
        let parent_dir = source_path.parent().ok_or_else(|| anyhow!("No parent directory"))?;
        
        // Create files in parent directory, not in tmp/
        let backup_file_path = parent_dir.join("backup.tar.gz");
        let meta_file_path = parent_dir.join("rdumper.backup.json");
        
        // Create metadata info first
        let database_config_info = DatabaseConfigInfo {
            id: database_config.id.clone(),
            name: database_config.name.clone(),
            host: database_config.host.clone(),
            port: database_config.port as u16,
            username: database_config.username.clone(),
            database_name: database_config.database_name.clone(),
        };
        
        let task_info = task.map(|t| TaskInfo {
            id: t.id.clone(),
            name: t.name.clone(),
            schedule: Some(t.cron_schedule.clone()),
            use_non_transactional: t.use_non_transactional,
        });
        
        // Determine used_database for this backup
        let used_database = if let Some(task) = task {
            let database_name = match &task.database_name {
                Some(db_name) => db_name.clone(),
                None => database_config.database_name.clone(),
            };
            Some(format!("{}/{}", database_config.name, database_name))
        } else {
            Some(format!("{}/{}", database_config.name, database_config.database_name))
        };

        // Create initial backup object
        let mut backup = Backup::new(
            database_config.database_name.clone(),
            database_config.id.clone(),
            task.map(|t| t.id.clone()),
            backup_file_path.to_string_lossy().to_string(),
            meta_file_path.to_string_lossy().to_string(),
            0, // Will be updated after tar.gz creation
            compression_type.to_string(),
            backup_type.to_string(),
        );
        backup.used_database = used_database;
        
        // Create initial metadata (without hash yet)
        let mut backup_metadata = BackupMetadata::new(&backup, database_config_info.clone(), task_info.clone());
        self.save_backup_metadata(&backup_metadata).await?;
        
        // Create tar.gz from source directory
        self.create_tar_gz_from_directory(source_dir, &backup_file_path).await?;
        
        // Get file size and calculate SHA-256 hash
        let metadata = fs::metadata(&backup_file_path).await?;
        let file_size = metadata.len() as i64;
        
        // Calculate file identifier (size + timestamp)
        let file_metadata = fs::metadata(&backup_file_path).await?;
        let file_modified = file_metadata.modified().unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH);
        let modified_timestamp = file_modified.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let ident = format!("size_{}_modified_{}", file_size, modified_timestamp);
        
        // Update backup with correct file size
        let backup = Backup::new(
            database_config.database_name.clone(),
            database_config.id.clone(),
            task.map(|t| t.id.clone()),
            backup_file_path.to_string_lossy().to_string(),
            meta_file_path.to_string_lossy().to_string(),
            file_size,
            compression_type.to_string(),
            backup_type.to_string(),
        );
        
        // Update metadata with correct file size and identifier
        backup_metadata.file_size = file_size;
        backup_metadata.ident = Some(ident);
        self.save_backup_metadata(&backup_metadata).await?;
        
        // Delete tmp directory after successful backup creation
        if let Err(e) = fs::remove_dir_all(source_dir).await {
            warn!("Failed to remove tmp directory {}: {}", source_dir, e);
        } else {
            info!("Successfully removed tmp directory: {}", source_dir);
        }
        
        Ok(backup)
    }

    /// Create a new backup entry (for uploads) - DEPRECATED: Use BackupProcess instead
    #[deprecated(note = "Use create_backup_process and BackupProcess::complete() instead")]
    pub async fn create_backup(
        &self,
        database_config: &DatabaseConfig,
        task: Option<&Task>,
        backup_type: &str,
        source_dir: &str, // Directory containing files to backup
        compression_type: &str,
    ) -> Result<Backup> {
        // Generate a human-readable directory name for the folder
        let directory_name = self.generate_backup_directory_name(database_config, task);
        let backup_dir = Path::new(&self.backup_base_dir).join(&directory_name);
        
        // Create backup directory
        fs::create_dir_all(&backup_dir).await?;
        
        // Create the final backup.tar.gz file
        let backup_file_path = backup_dir.join("backup.tar.gz");
        let meta_file_path = backup_dir.join("rdumper.backup.json");
        
        // Create tar.gz from source directory
        self.create_tar_gz_from_directory(source_dir, &backup_file_path).await?;
        
        // Get file size
        let metadata = fs::metadata(&backup_file_path).await?;
        let file_size = metadata.len() as i64;
        
        // Create backup entry
        let backup = Backup::new(
            database_config.database_name.clone(),
            database_config.id.clone(),
            task.map(|t| t.id.clone()),
            backup_file_path.to_string_lossy().to_string(),
            meta_file_path.to_string_lossy().to_string(),
            file_size,
            compression_type.to_string(),
            backup_type.to_string(),
        );
        
        // Create and save metadata
        let database_config_info = DatabaseConfigInfo {
            id: database_config.id.clone(),
            name: database_config.name.clone(),
            host: database_config.host.clone(),
            port: database_config.port as u16,
            username: database_config.username.clone(),
            database_name: database_config.database_name.clone(),
        };
        
        let task_info = task.map(|t| TaskInfo {
            id: t.id.clone(),
            name: t.name.clone(),
            schedule: Some(t.cron_schedule.clone()),
            use_non_transactional: t.use_non_transactional,
        });
        
        let backup_metadata = BackupMetadata::new(&backup, database_config_info, task_info);
        self.save_backup_metadata(&backup_metadata).await?;

        info!("Backup created: {}", backup.file_path);
        Ok(backup)
    }

    /// Delete a backup and its metadata
    pub async fn delete_backup(&self, backup: &Backup) -> Result<()> {
        // Delete backup file
        if std::path::Path::new(&backup.file_path).exists() {
            fs::remove_file(&backup.file_path).await?;
        }
        
        // Delete metadata file
        if std::path::Path::new(&backup.meta_path).exists() {
            fs::remove_file(&backup.meta_path).await?;
        }
        
        // Try to remove empty parent directory
        if let Some(parent) = std::path::Path::new(&backup.file_path).parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                if entries.count() == 0 {
                    let _ = fs::remove_dir(parent).await;
                }
            }
        }
        
        Ok(())
    }

    /// Get backup statistics
    pub async fn get_backup_stats(&self) -> Result<BackupStats> {
        let backups = self.scan_backups().await?;
        
        let total_count = backups.len();
        let total_size: i64 = backups.iter().map(|b| b.file_size).sum();
        
        let mut by_type = std::collections::HashMap::new();
        let mut by_database = std::collections::HashMap::new();
        
        for backup in &backups {
            *by_type.entry(backup.backup_type.clone()).or_insert(0) += 1;
            *by_database.entry(backup.database_name.clone()).or_insert(0) += 1;
        }
        
        Ok(BackupStats {
            total_count,
            total_size,
            by_type,
            by_database,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_count: usize,
    pub total_size: i64,
    pub by_type: std::collections::HashMap<String, usize>,
    pub by_database: std::collections::HashMap<String, usize>,
}
