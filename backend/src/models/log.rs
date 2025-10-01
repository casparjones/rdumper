use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Log {
    pub id: String,
    pub log_type: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub message: String,
    pub level: String,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LogType {
    Connection,
    Task,
    Worker,
    Job,
    System,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogType::Connection => write!(f, "connection"),
            LogType::Task => write!(f, "task"),
            LogType::Worker => write!(f, "worker"),
            LogType::Job => write!(f, "job"),
            LogType::System => write!(f, "system"),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLogRequest {
    pub log_type: LogType,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub message: String,
    pub level: LogLevel,
    pub metadata: Option<serde_json::Value>,
}

impl Log {
    pub fn new(req: CreateLogRequest) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            log_type: req.log_type.to_string(),
            entity_type: req.entity_type,
            entity_id: req.entity_id,
            message: req.message,
            level: req.level.to_string(),
            metadata: req.metadata.map(|v| v.to_string()),
            created_at: Utc::now(),
        }
    }
}
