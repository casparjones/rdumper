use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::fs;
use tokio::fs as async_fs;
use chrono::Utc;
use serde::{Serialize, Deserialize};

use crate::models::{DatabaseConfig, Task, BackupMetadata, DatabaseConfigInfo, TaskInfo};

#[derive(Debug)]
pub struct BackupProcess {
    pub id: String,
    pub root_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub meta_file: PathBuf,
    pub database_config: DatabaseConfig,
    pub task: Option<Task>,
    pub backup_type: String,
    pub compression_type: String,
}

impl BackupProcess {
    /// Create a new backup process
    pub fn new(
        id: String,
        root_dir: PathBuf,
        database_config: DatabaseConfig,
        task: Option<Task>,
        backup_type: String,
        compression_type: String,
    ) -> Self {
        let tmp_dir = root_dir.join("tmp");
        let meta_file = root_dir.join("rdumper.backup.json");
        
        Self {
            id,
            root_dir,
            tmp_dir,
            meta_file,
            database_config,
            task,
            backup_type,
            compression_type,
        }
    }
    
    /// Initialize the backup process by creating directories and metadata
    pub async fn initialize(&self) -> Result<()> {
        // Create root directory
        async_fs::create_dir_all(&self.root_dir).await?;
        
        // Create tmp directory
        async_fs::create_dir_all(&self.tmp_dir).await?;
        
        // Create initial metadata
        self.create_initial_metadata().await?;
        
        Ok(())
    }
    
    /// Get the tmp directory path for mydumper output
    pub fn tmp_dir(&self) -> &Path {
        &self.tmp_dir
    }
    
    /// Complete the backup process by creating archive and cleaning up
    pub async fn complete(&mut self) -> Result<String> {
        // Create backup archive
        let archive_path = self.create_archive().await?;
        
        // Get file size and modification time
        let metadata = async_fs::metadata(&archive_path).await?;
        let file_size = metadata.len() as i64;
        let file_modified = metadata.modified().unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH);
        
        // Update metadata with file information (no hash needed)
        self.update_metadata_fast(&archive_path, file_size, file_modified).await?;
        
        // Clean up tmp directory immediately
        self.cleanup_tmp().await?;
        
        // Return the archive path as string
        Ok(archive_path.to_string_lossy().to_string())
    }
    
    /// Create initial metadata file
    async fn create_initial_metadata(&self) -> Result<()> {
        let database_config_info = DatabaseConfigInfo {
            id: self.database_config.id.clone(),
            name: self.database_config.name.clone(),
            host: self.database_config.host.clone(),
            port: self.database_config.port as u16,
            username: self.database_config.username.clone(),
            database_name: self.database_config.database_name.clone(),
        };
        
        let task_info = self.task.as_ref().map(|t| TaskInfo {
            id: t.id.clone(),
            name: t.name.clone(),
            schedule: Some(t.cron_schedule.clone()),
            use_non_transactional: t.use_non_transactional,
        });
        
        // Determine used_database for this backup
        let used_database = if let Some(task) = &self.task {
            let database_name = match &task.database_name {
                Some(db_name) => db_name.clone(),
                None => self.database_config.database_name.clone(),
            };
            Some(format!("{}/{}", self.database_config.name, database_name))
        } else {
            Some(format!("{}/{}", self.database_config.name, self.database_config.database_name))
        };

        let backup_metadata = BackupMetadata {
            id: self.id.clone(),
            database_name: self.database_config.database_name.clone(),
            database_config_id: self.database_config.id.clone(),
            task_id: self.task.as_ref().map(|t| t.id.clone()),
            used_database,
            file_path: String::new(), // Will be set when archive is created
            meta_path: self.meta_file.to_string_lossy().to_string(),
            file_size: 0, // Will be set when archive is created
            compression_type: self.compression_type.clone(),
            created_at: Utc::now().to_rfc3339(),
            backup_type: self.backup_type.clone(),
            ident: None, // Will be set when archive is created
            database_config: database_config_info,
            task_info,
        };
        
        let content = serde_json::to_string_pretty(&backup_metadata)?;
        async_fs::write(&self.meta_file, content).await?;
        
        Ok(())
    }
    
    /// Create backup archive from tmp directory
    async fn create_archive(&self) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let archive_name = format!("{}-{}.{}", 
            self.database_config.database_name, 
            timestamp,
            self.get_archive_extension()
        );
        let archive_path = self.root_dir.join(&archive_name);
        
        // Create tar archive
        self.create_tar_archive(&archive_path).await?;
        
        Ok(archive_path)
    }
    
    /// Get archive extension based on compression type
    fn get_archive_extension(&self) -> &'static str {
        match self.compression_type.as_str() {
            "gzip" => "tar.gz",
            "zstd" => "tar.zst",
            "none" => "tar",
            _ => "tar.gz", // Default to gzip
        }
    }
    
    /// Create tar archive with appropriate compression
    async fn create_tar_archive(&self, output_path: &Path) -> Result<()> {
        use tokio::process::Command;
        
        // Wait a moment to ensure all files are written
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        
        let mut cmd = Command::new("tar");
        
        match self.compression_type.as_str() {
            "gzip" => {
                cmd.args(&["-czf", output_path.to_str().unwrap()]);
            },
            "zstd" => {
                cmd.args(&["-c", "--zstd", "-f", output_path.to_str().unwrap()]);
            },
            "none" => {
                cmd.args(&["-cf", output_path.to_str().unwrap()]);
            },
            _ => {
                cmd.args(&["-czf", output_path.to_str().unwrap()]);
            }
        }
        
        cmd.args(&["-C", self.tmp_dir.to_str().unwrap(), "--warning=no-file-changed", "."]);
        
        let status = cmd.status().await?;
        
        if !status.success() {
            return Err(anyhow!("Failed to create tar archive"));
        }
        
        Ok(())
    }
    
    
    /// Update metadata with final information
    async fn update_metadata(&self, archive_path: &Path, file_size: i64, sha256_hash: String) -> Result<()> {
        let content = async_fs::read_to_string(&self.meta_file).await?;
        let mut metadata: BackupMetadata = serde_json::from_str(&content)?;
        
        // Update with final information
        metadata.file_path = archive_path.to_string_lossy().to_string();
        metadata.file_size = file_size;
        metadata.ident = Some(sha256_hash);
        
        let updated_content = serde_json::to_string_pretty(&metadata)?;
        async_fs::write(&self.meta_file, updated_content).await?;
        
        Ok(())
    }
    
    /// Update metadata without hash (for immediate response)
    async fn update_metadata_without_hash(&self, archive_path: &Path, file_size: i64) -> Result<()> {
        let content = async_fs::read_to_string(&self.meta_file).await?;
        let mut metadata: BackupMetadata = serde_json::from_str(&content)?;
        
        // Update with file information (without hash)
        metadata.file_path = archive_path.to_string_lossy().to_string();
        metadata.file_size = file_size;
        // ident remains None for now
        
        let updated_content = serde_json::to_string_pretty(&metadata)?;
        async_fs::write(&self.meta_file, updated_content).await?;
        
        Ok(())
    }
    
    /// Update metadata fast (no hash calculation)
    async fn update_metadata_fast(&self, archive_path: &Path, file_size: i64, file_modified: std::time::SystemTime) -> Result<()> {
        let content = async_fs::read_to_string(&self.meta_file).await?;
        let mut metadata: BackupMetadata = serde_json::from_str(&content)?;
        
        // Update with file information (no hash needed)
        metadata.file_path = archive_path.to_string_lossy().to_string();
        metadata.file_size = file_size;
        // Use file modification time as a simple integrity check
        let modified_timestamp = file_modified.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        metadata.ident = Some(format!("size_{}_modified_{}", file_size, modified_timestamp));
        
        let updated_content = serde_json::to_string_pretty(&metadata)?;
        async_fs::write(&self.meta_file, updated_content).await?;
        
        Ok(())
    }
    
    /// Clean up tmp directory
    async fn cleanup_tmp(&self) -> Result<()> {
        if self.tmp_dir.exists() {
            async_fs::remove_dir_all(&self.tmp_dir).await?;
        }
        Ok(())
    }
}
