use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableProgress {
    pub name: String,
    pub status: TableStatus,
    pub progress_percent: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TableStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedProgress {
    pub job_id: String,
    pub overall_progress: u32,
    pub total_tables: u32,
    pub completed_tables: u32,
    pub in_progress_tables: u32,
    pub pending_tables: u32,
    pub skipped_tables: u32,
    pub error_tables: u32,
    pub tables: Vec<TableProgress>,
    pub excluded_tables: Vec<String>,
    pub database_name: String,
    pub started_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdumperMeta {
    pub count: u32,
    pub tables: Vec<String>,
    pub excluded_tables: Vec<String>,
    pub database_name: String,
    pub started_at: String,
}
