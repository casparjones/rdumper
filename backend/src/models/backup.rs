use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub id: String,
    pub database_name: String,
    pub database_config_id: String,
    pub task_id: Option<String>,
    pub file_path: String,
    pub meta_path: String,
    pub file_size: i64,
    pub compression_type: String,
    pub created_at: DateTime<Utc>,
    pub backup_type: String, // "manual", "scheduled", "uploaded"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: String,
    pub database_name: String,
    pub database_config_id: String,
    pub task_id: Option<String>,
    pub file_path: String,
    pub meta_path: String,
    pub file_size: i64,
    pub compression_type: String,
    pub created_at: DateTime<Utc>,
    pub backup_type: String,
    pub sha256_hash: Option<String>,
    pub database_config: DatabaseConfigInfo,
    pub task_info: Option<TaskInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfigInfo {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub database_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub schedule: Option<String>,
    pub use_non_transactional: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBackupRequest {
    pub database_config_id: String,
    pub task_id: Option<String>,
    pub file_path: String,
    pub file_size: i64,
    pub compression_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreRequest {
    pub backup_id: String,
    pub target_database_config_id: Option<String>,
    pub new_database_name: Option<String>,
    pub overwrite_existing: bool,
}

impl Backup {
    pub fn new(
        database_name: String,
        database_config_id: String,
        task_id: Option<String>,
        file_path: String,
        meta_path: String,
        file_size: i64,
        compression_type: String,
        backup_type: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            database_name,
            database_config_id,
            task_id,
            file_path,
            meta_path,
            file_size,
            compression_type,
            created_at: Utc::now(),
            backup_type,
        }
    }

    pub fn filename(&self) -> Option<&str> {
        Path::new(&self.file_path).file_name()?.to_str()
    }

    pub fn meta_filename(&self) -> Option<&str> {
        Path::new(&self.meta_path).file_name()?.to_str()
    }

    pub fn backup_folder(&self) -> Option<&str> {
        Path::new(&self.file_path).parent()?.file_name()?.to_str()
    }

    pub fn file_size_human(&self) -> String {
        let size = self.file_size as f64;
        let units = ["B", "KB", "MB", "GB", "TB"];
        let mut size = size;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < units.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, units[unit_index])
        } else {
            format!("{:.2} {}", size, units[unit_index])
        }
    }

    /// Load backup metadata from filesystem
    pub async fn load_metadata(&self) -> Result<BackupMetadata, Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(&self.meta_path).await?;
        let metadata: BackupMetadata = serde_json::from_str(&content)?;
        Ok(metadata)
    }

    /// Save backup metadata to filesystem
    pub async fn save_metadata(&self, metadata: &BackupMetadata) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(metadata)?;
        tokio::fs::write(&self.meta_path, content).await?;
        Ok(())
    }

    /// Check if backup files exist on filesystem
    pub fn exists(&self) -> bool {
        Path::new(&self.file_path).exists() && Path::new(&self.meta_path).exists()
    }

    /// Get backup age in days
    pub fn age_days(&self) -> i64 {
        let now = Utc::now();
        (now - self.created_at).num_days()
    }
}

impl BackupMetadata {
    pub fn new(
        backup: &Backup,
        database_config: DatabaseConfigInfo,
        task_info: Option<TaskInfo>,
    ) -> Self {
        Self {
            id: backup.id.clone(),
            database_name: backup.database_name.clone(),
            database_config_id: backup.database_config_id.clone(),
            task_id: backup.task_id.clone(),
            file_path: backup.file_path.clone(),
            meta_path: backup.meta_path.clone(),
            file_size: backup.file_size,
            compression_type: backup.compression_type.clone(),
            created_at: backup.created_at,
            backup_type: backup.backup_type.clone(),
            sha256_hash: None, // Will be set when calculating hash
            database_config,
            task_info,
        }
    }

    /// Calculate SHA-256 hash of the backup file
    pub fn calculate_sha256_hash(&mut self, file_path: &str) -> anyhow::Result<String> {
        use std::fs::File;
        use std::io::Read;
        use sha2::{Sha256, Digest};

        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = format!("{:x}", hasher.finalize());
        self.sha256_hash = Some(hash.clone());
        Ok(hash)
    }
}