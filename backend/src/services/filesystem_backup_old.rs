use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{warn, info};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{Backup, BackupMetadata, DatabaseConfigInfo, TaskInfo, DatabaseConfig, Task};

#[derive(Debug, Clone)]
pub struct FilesystemBackupService {
    backup_base_dir: String,
}

impl FilesystemBackupService {
    pub fn new(backup_base_dir: String) -> Self {
        Self { backup_base_dir }
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
        let mut entries = fs::read_dir(dir_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively scan subdirectories
                self.scan_directory_recursive(&path, backups).await?;
            } else if path.is_file() {
                // Check if this is a backup file
                if let Some(backup_file) = self.is_backup_file(&path) {
                    // Look for corresponding metadata file
                    let meta_file = self.find_metadata_file(&path).await?;
                    
                    if let Some(meta_path) = meta_file {
                        // Load existing metadata
                        match self.load_backup_metadata(&meta_path).await {
                            Ok(metadata) => {
                                let backup = Backup {
                                    id: metadata.id,
                                    database_name: metadata.database_name,
                                    database_config_id: metadata.database_config_id,
                                    task_id: metadata.task_id,
                                    file_path: path.to_string_lossy().to_string(),
                                    meta_path: meta_path.to_string_lossy().to_string(),
                                    file_size: metadata.file_size,
                                    compression_type: metadata.compression_type,
                                    created_at: metadata.created_at,
                                    backup_type: metadata.backup_type,
                                };
                                backups.push(backup);
                            }
                            Err(_) => {
                                // Create dummy metadata
                                let backup = self.create_dummy_backup(&path, &meta_path).await?;
                                backups.push(backup);
                            }
                        }
                    } else {
                        // Create metadata file with dummy data
                        let meta_path = self.create_metadata_file_for_backup(&path).await?;
                        let backup = self.create_dummy_backup(&path, &meta_path).await?;
                        backups.push(backup);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Scan a specific backup folder for backup files
    async fn scan_backup_folder(&self, folder_path: &Path, job_id: &str) -> Result<Vec<Backup>> {
        let mut backups = Vec::new();
        
        // Look for backup.tar.gz and rdumper.backup.json in the folder
        let backup_file = folder_path.join("backup.tar.gz");
        let meta_file = folder_path.join("rdumper.backup.json");
        
        info!("Scanning folder: {}, backup_file exists: {}, meta_file exists: {}", 
              folder_path.display(), backup_file.exists(), meta_file.exists());
        
        if backup_file.exists() && meta_file.exists() {
            // Try to load metadata to get backup info
            match self.load_backup_metadata(&meta_file).await {
                Ok(metadata) => {
                    let backup = Backup {
                        id: metadata.id,
                        database_name: metadata.database_name,
                        database_config_id: metadata.database_config_id,
                        task_id: metadata.task_id,
                        file_path: backup_file.to_string_lossy().to_string(),
                        meta_path: meta_file.to_string_lossy().to_string(),
                        file_size: metadata.file_size,
                        compression_type: metadata.compression_type,
                        created_at: metadata.created_at,
                        backup_type: metadata.backup_type,
                    };
                    backups.push(backup);
                }
                Err(e) => {
                    warn!("Failed to load metadata for {}: {}", meta_file.display(), e);
                    // Create a minimal backup entry
                    let backup = self.create_minimal_backup(&backup_file, &meta_file, job_id).await?;
                    backups.push(backup);
                }
            }
        } else {
            warn!("Backup folder missing required files: backup.tar.gz or rdumper.json in {}", folder_path.display());
        }
        
        Ok(backups)
    }

    /// Create a minimal backup entry when metadata is missing
    async fn create_minimal_backup(&self, file_path: &Path, meta_path: &Path, job_id: &str) -> Result<Backup> {
        let metadata = fs::metadata(file_path).await?;
        let file_size = metadata.len() as i64;
        
        // Determine compression type from file extension
        let compression_type = if file_path.to_string_lossy().ends_with(".tar.zst") {
            "zstd".to_string()
        } else if file_path.to_string_lossy().ends_with(".tar.gz") {
            "gzip".to_string()
        } else {
            "none".to_string()
        };
        
        // Extract timestamp from filename
        let filename = file_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid filename"))?;
        
        let _created_at = self.extract_timestamp_from_filename(filename)
            .unwrap_or_else(|| Utc::now());
        
        // Try to extract database name from job_id or use job_id as fallback
        let database_name = job_id.to_string(); // For now, use job_id as database name
        
        let backup = Backup::new(
            database_name,
            "unknown".to_string(), // Will be updated when metadata is available
            None,
            file_path.to_string_lossy().to_string(),
            meta_path.to_string_lossy().to_string(),
            file_size,
            compression_type,
            "unknown".to_string(),
        );
        
        Ok(backup)
    }

    /// Load backup metadata from JSON file
    async fn load_backup_metadata(&self, meta_path: &Path) -> Result<BackupMetadata> {
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
        source_dir: &str, // Directory containing mydumper output files
        compression_type: &str,
    ) -> Result<Backup> {
        let source_path = Path::new(source_dir);
        let backup_file_path = source_path.join("backup.tar.gz");
        let meta_file_path = source_path.join("rdumper.backup.json");
        
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
        
        Ok(backup)
    }

    /// Delete a backup and its metadata
    pub async fn delete_backup(&self, backup: &Backup) -> Result<()> {
        // Delete backup file
        if Path::new(&backup.file_path).exists() {
            fs::remove_file(&backup.file_path).await?;
        }
        
        // Delete metadata file
        if Path::new(&backup.meta_path).exists() {
            fs::remove_file(&backup.meta_path).await?;
        }
        
        // Check if backup folder is empty and delete it
        if let Some(folder_path) = Path::new(&backup.file_path).parent() {
            if let Ok(mut entries) = fs::read_dir(folder_path).await {
                let mut has_files = false;
                while let Some(_) = entries.next_entry().await? {
                    has_files = true;
                    break;
                }
                
                if !has_files {
                    fs::remove_dir(folder_path).await?;
                }
            }
        }
        
        Ok(())
    }

    /// Check if folder name is a job ID (UUID format)
    fn is_job_folder(&self, folder_name: &str) -> bool {
        // Check if it's a UUID format (8-4-4-4-12 characters with hyphens)
        let parts: Vec<&str> = folder_name.split('-').collect();
        parts.len() == 5 
            && parts[0].len() == 8 
            && parts[1].len() == 4 
            && parts[2].len() == 4 
            && parts[3].len() == 4 
            && parts[4].len() == 12
            && folder_name.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
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
        
        // Create dummy metadata
        let dummy_metadata = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "database_name": "unknown",
            "database_config_id": "unknown",
            "task_id": null,
            "file_path": backup_path.to_string_lossy(),
            "meta_path": meta_file.to_string_lossy(),
            "file_size": 0,
            "compression_type": "unknown",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "backup_type": "external",
            "database_config": {
                "id": "unknown",
                "name": "Unknown Database",
                "host": "unknown",
                "port": 3306,
                "username": "unknown",
                "database_name": "unknown"
            },
            "task_info": null
        });
        
        fs::write(&meta_file, serde_json::to_string_pretty(&dummy_metadata)?).await?;
        Ok(meta_file)
    }

    /// Create dummy backup entry
    async fn create_dummy_backup(&self, backup_path: &Path, meta_path: &Path) -> Result<Backup> {
        let metadata = fs::metadata(backup_path).await?;
        let file_size = metadata.len() as i64;
        
        // Determine compression type from file extension
        let compression_type = if backup_path.to_string_lossy().ends_with(".tar.zst") {
            "zstd".to_string()
        } else if backup_path.to_string_lossy().ends_with(".tar.gz") {
            "gzip".to_string()
        } else if backup_path.to_string_lossy().ends_with(".tar") {
            "none".to_string()
        } else {
            "unknown".to_string()
        };
        
        // Extract database name from path or use "unknown"
        let database_name = backup_path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let backup = Backup::new(
            database_name,
            "unknown".to_string(),
            None,
            backup_path.to_string_lossy().to_string(),
            meta_path.to_string_lossy().to_string(),
            file_size,
            compression_type,
            "external".to_string(),
        );
        
        Ok(backup)
    }

    /// Extract timestamp from filename
    fn extract_timestamp_from_filename(&self, filename: &str) -> Option<DateTime<Utc>> {
        // Look for pattern: YYYYMMDD_HHMMSS
        let re = regex::Regex::new(r"(\d{8}_\d{6})").ok()?;
        if let Some(captures) = re.captures(filename) {
            let timestamp_str = &captures[1];
            if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y%m%d_%H%M%S") {
                return Some(parsed.and_utc());
            }
        }
        None
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

    /// Create a new backup entry (for uploads)
    pub async fn create_backup(
        &self,
        database_config: &DatabaseConfig,
        task: Option<&Task>,
        backup_type: &str,
        source_dir: &str, // Directory containing files to backup
        compression_type: &str,
    ) -> Result<Backup> {
        // Generate a unique job ID for the folder name
        let job_id = uuid::Uuid::new_v4().to_string();
        let backup_dir = Path::new(&self.backup_base_dir).join(&job_id);
        
        // Create backup directory
        fs::create_dir_all(&backup_dir).await?;
        
        // Create the final backup.tar.gz file
        let backup_file_path = backup_dir.join("backup.tar.gz");
        let meta_file_path = backup_dir.join("rdumper.json");
        
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_count: usize,
    pub total_size: i64,
    pub by_type: std::collections::HashMap<String, usize>,
    pub by_database: std::collections::HashMap<String, usize>,
}
