use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Backup {
    pub id: String,
    pub database_config_id: String,
    pub task_id: Option<String>,
    pub file_path: String,
    pub file_size: i64,
    pub compression_type: String,
    pub created_at: DateTime<Utc>,
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
    pub fn new(req: CreateBackupRequest) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            database_config_id: req.database_config_id,
            task_id: req.task_id,
            file_path: req.file_path,
            file_size: req.file_size,
            compression_type: req.compression_type,
            created_at: Utc::now(),
        }
    }

    // pub fn file_size_human(&self) -> String {
    //     let size = self.file_size as f64;
    //     let units = ["B", "KB", "MB", "GB", "TB"];
    //     let mut size = size;
    //     let mut unit_index = 0;

    //     while size >= 1024.0 && unit_index < units.len() - 1 {
    //         size /= 1024.0;
    //         unit_index += 1;
    //     }

    //     if unit_index == 0 {
    //         format!("{} {}", size as u64, units[unit_index])
    //     } else {
    //         format!("{:.2} {}", size, units[unit_index])
    //     }
    // }

    pub fn filename(&self) -> Option<&str> {
        std::path::Path::new(&self.file_path).file_name()?.to_str()
    }

    // pub fn extension(&self) -> Option<&str> {
    //     std::path::Path::new(&self.file_path).extension()?.to_str()
    // }
}