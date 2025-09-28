use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "zstd")]
    Zstd,
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::Gzip
    }
}

impl std::fmt::Display for CompressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionType::None => write!(f, "none"),
            CompressionType::Gzip => write!(f, "gzip"),
            CompressionType::Zstd => write!(f, "zstd"),
        }
    }
}

impl std::str::FromStr for CompressionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(CompressionType::None),
            "gzip" => Ok(CompressionType::Gzip),
            "zstd" => Ok(CompressionType::Zstd),
            _ => Err(format!("Invalid compression type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub database_config_id: String,
    pub cron_schedule: String,
    pub compression_type: String,
    pub cleanup_days: i32,
    pub use_non_transactional: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub database_config_id: String,
    pub cron_schedule: String,
    pub compression_type: Option<CompressionType>,
    pub cleanup_days: Option<i32>,
    pub use_non_transactional: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub cron_schedule: Option<String>,
    pub compression_type: Option<CompressionType>,
    pub cleanup_days: Option<i32>,
    pub use_non_transactional: Option<bool>,
    pub is_active: Option<bool>,
}

impl Task {
    pub fn new(req: CreateTaskRequest) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            database_config_id: req.database_config_id,
            cron_schedule: req.cron_schedule,
            compression_type: req.compression_type.unwrap_or_default().to_string(),
            cleanup_days: req.cleanup_days.unwrap_or(30),
            use_non_transactional: req.use_non_transactional.unwrap_or(false),
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update(&mut self, req: UpdateTaskRequest) {
        if let Some(name) = req.name {
            self.name = name;
        }
        if let Some(cron_schedule) = req.cron_schedule {
            self.cron_schedule = cron_schedule;
        }
        if let Some(compression_type) = req.compression_type {
            self.compression_type = compression_type.to_string();
        }
        if let Some(cleanup_days) = req.cleanup_days {
            self.cleanup_days = cleanup_days;
        }
        if let Some(use_non_transactional) = req.use_non_transactional {
            self.use_non_transactional = use_non_transactional;
        }
        if let Some(is_active) = req.is_active {
            self.is_active = is_active;
        }
        self.updated_at = Utc::now();
    }

    pub fn compression_type(&self) -> Result<CompressionType, String> {
        self.compression_type.parse()
    }
}