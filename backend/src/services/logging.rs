use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{info, warn, error};
use crate::models::{Log, LogType, LogLevel, CreateLogRequest};

pub struct LoggingService {
    db_pool: Arc<SqlitePool>,
}

impl LoggingService {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { db_pool: pool }
    }

    /// Log a message to the database
    pub async fn log(&self, req: CreateLogRequest) -> Result<(), sqlx::Error> {
        let log_entry = Log::new(req);
        
        sqlx::query(
            r#"
            INSERT INTO logs (id, log_type, entity_type, entity_id, message, level, metadata, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&log_entry.id)
        .bind(&log_entry.log_type)
        .bind(&log_entry.entity_type)
        .bind(&log_entry.entity_id)
        .bind(&log_entry.message)
        .bind(&log_entry.level)
        .bind(&log_entry.metadata)
        .bind(&log_entry.created_at)
        .execute(&*self.db_pool)
        .await?;

        // Also log to console based on level
        match log_entry.level.as_str() {
            "debug" => tracing::debug!("[{}] {}", log_entry.log_type, log_entry.message),
            "info" => info!("[{}] {}", log_entry.log_type, log_entry.message),
            "warn" => warn!("[{}] {}", log_entry.log_type, log_entry.message),
            "error" => error!("[{}] {}", log_entry.log_type, log_entry.message),
            _ => info!("[{}] {}", log_entry.log_type, log_entry.message),
        }

        Ok(())
    }

    /// Log connection events
    pub async fn log_connection(&self, entity_id: &str, message: &str, level: LogLevel) -> Result<(), sqlx::Error> {
        self.log(CreateLogRequest {
            log_type: LogType::Connection,
            entity_type: "database_config".to_string(),
            entity_id: Some(entity_id.to_string()),
            message: message.to_string(),
            level,
            metadata: None,
        }).await
    }

    /// Log task events
    pub async fn log_task(&self, entity_id: &str, message: &str, level: LogLevel) -> Result<(), sqlx::Error> {
        self.log(CreateLogRequest {
            log_type: LogType::Task,
            entity_type: "task".to_string(),
            entity_id: Some(entity_id.to_string()),
            message: message.to_string(),
            level,
            metadata: None,
        }).await
    }

    /// Log worker events
    pub async fn log_worker(&self, message: &str, level: LogLevel) -> Result<(), sqlx::Error> {
        self.log(CreateLogRequest {
            log_type: LogType::Worker,
            entity_type: "worker".to_string(),
            entity_id: None,
            message: message.to_string(),
            level,
            metadata: None,
        }).await
    }

    /// Log job events
    pub async fn log_job(&self, entity_id: &str, message: &str, level: LogLevel) -> Result<(), sqlx::Error> {
        self.log(CreateLogRequest {
            log_type: LogType::Job,
            entity_type: "job".to_string(),
            entity_id: Some(entity_id.to_string()),
            message: message.to_string(),
            level,
            metadata: None,
        }).await
    }

    /// Log system events
    pub async fn log_system(&self, message: &str, level: LogLevel) -> Result<(), sqlx::Error> {
        self.log(CreateLogRequest {
            log_type: LogType::System,
            entity_type: "system".to_string(),
            entity_id: None,
            message: message.to_string(),
            level,
            metadata: None,
        }).await
    }

    /// Log system events with entity information
    pub async fn log_system_with_entity(&self, entity_type: &str, entity_id: &str, message: &str, level: LogLevel) -> Result<(), sqlx::Error> {
        self.log(CreateLogRequest {
            log_type: LogType::System,
            entity_type: entity_type.to_string(),
            entity_id: Some(entity_id.to_string()),
            message: message.to_string(),
            level,
            metadata: None,
        }).await
    }

    /// Get logs with optional filtering
    pub async fn get_logs(
        &self,
        log_type: Option<LogType>,
        entity_type: Option<String>,
        entity_id: Option<String>,
        level: Option<LogLevel>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Log>, sqlx::Error> {
        let mut query = "SELECT * FROM logs WHERE 1=1".to_string();
        let mut params = Vec::new();
        let mut param_count = 0;

        if let Some(log_type) = log_type {
            param_count += 1;
            query.push_str(&format!(" AND log_type = ?{}", param_count));
            params.push(log_type.to_string());
        }

        if let Some(entity_type) = entity_type {
            param_count += 1;
            query.push_str(&format!(" AND entity_type = ?{}", param_count));
            params.push(entity_type);
        }

        if let Some(entity_id) = entity_id {
            param_count += 1;
            query.push_str(&format!(" AND entity_id = ?{}", param_count));
            params.push(entity_id);
        }

        if let Some(level) = level {
            param_count += 1;
            query.push_str(&format!(" AND level = ?{}", param_count));
            params.push(level.to_string());
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            param_count += 1;
            query.push_str(&format!(" LIMIT ?{}", param_count));
            params.push(limit.to_string());
        }

        if let Some(offset) = offset {
            param_count += 1;
            query.push_str(&format!(" OFFSET ?{}", param_count));
            params.push(offset.to_string());
        }

        let mut sql_query = sqlx::query_as::<_, Log>(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }

        sql_query.fetch_all(&*self.db_pool).await
    }

    /// Clean up old logs (older than specified days)
    pub async fn cleanup_old_logs(&self, days: u32) -> Result<u64, sqlx::Error> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days as i64);
        
        let result = sqlx::query("DELETE FROM logs WHERE created_at < ?")
            .bind(cutoff_date)
            .execute(&*self.db_pool)
            .await?;

        Ok(result.rows_affected())
    }
}
