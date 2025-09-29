use anyhow::{anyhow, Result};
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};
use chrono::Utc;
use regex::Regex;

use crate::models::progress::{DetailedProgress, TableProgress, TableStatus, RdumperMeta};

pub struct ProgressTracker {
    log_dir: String,
}

impl ProgressTracker {
    pub fn new(log_dir: String) -> Self {
        Self { log_dir }
    }

    /// Load detailed progress for a job
    pub async fn load_detailed_progress(&self, job_id: &str) -> Result<DetailedProgress> {
        let meta_file = format!("{}/rdumper.meta.json", self.log_dir);
        let log_file = format!("{}/mydumper.log", self.log_dir);

        // Load metadata
        let meta_content = fs::read_to_string(&meta_file).await?;
        let meta: RdumperMeta = serde_json::from_str(&meta_content)?;

        // Load log content
        let log_content = fs::read_to_string(&log_file).await?;

        // Parse table progress from log
        let mut tables = self.parse_table_progress(&log_content, &meta.tables).await?;
        
        // Add excluded tables as skipped
        for table_name in &meta.excluded_tables {
            tables.push(TableProgress {
                name: table_name.clone(),
                status: TableStatus::Skipped,
                progress_percent: None,
                started_at: None,
                completed_at: Some(Utc::now()),
                error_message: Some("Non-InnoDB table, excluded from backup".to_string()),
            });
        }

        // Calculate overall progress
        let total_tables = tables.len() as u32;
        let completed_tables = tables.iter().filter(|t| matches!(t.status, TableStatus::Completed)).count() as u32;
        let in_progress_tables = tables.iter().filter(|t| matches!(t.status, TableStatus::InProgress)).count() as u32;
        let pending_tables = tables.iter().filter(|t| matches!(t.status, TableStatus::Pending)).count() as u32;
        let skipped_tables = tables.iter().filter(|t| matches!(t.status, TableStatus::Skipped)).count() as u32;
        let error_tables = tables.iter().filter(|t| matches!(t.status, TableStatus::Error)).count() as u32;

        let overall_progress = if total_tables > 0 {
            ((completed_tables + skipped_tables) as f32 / total_tables as f32 * 100.0) as u32
        } else {
            0
        };

        Ok(DetailedProgress {
            job_id: job_id.to_string(),
            overall_progress,
            total_tables,
            completed_tables,
            in_progress_tables,
            pending_tables,
            skipped_tables,
            error_tables,
            tables,
            excluded_tables: meta.excluded_tables,
            database_name: meta.database_name,
            started_at: meta.started_at.parse().unwrap_or_else(|_| Utc::now()),
            last_updated: Utc::now(),
        })
    }

    /// Parse table progress from mydumper log
    async fn parse_table_progress(&self, log_content: &str, table_names: &[String]) -> Result<Vec<TableProgress>> {
        let mut tables = Vec::new();
        
        // Regex patterns for different log entries
        let schema_pattern = Regex::new(r"dumping schema for `([^`]+)`")?;
        let data_pattern = Regex::new(r"`([^`]+)` \[(\d+)%\]")?;
        let completed_pattern = Regex::new(r"`([^`]+)` \[100%\]")?;
        let error_pattern = Regex::new(r"ERROR.*`([^`]+)`")?;

        // Initialize all tables as pending
        for table_name in table_names {
            tables.push(TableProgress {
                name: table_name.clone(),
                status: TableStatus::Pending,
                progress_percent: None,
                started_at: None,
                completed_at: None,
                error_message: None,
            });
        }

        // Parse log lines
        for line in log_content.lines() {
            // Check for schema dumping (table started)
            if let Some(caps) = schema_pattern.captures(line) {
                let table_name = caps.get(1).unwrap().as_str();
                if let Some(table) = tables.iter_mut().find(|t| t.name == table_name) {
                    table.status = TableStatus::InProgress;
                    table.started_at = Some(Utc::now());
                }
            }

            // Check for data progress
            if let Some(caps) = data_pattern.captures(line) {
                let table_name = caps.get(1).unwrap().as_str();
                let progress = caps.get(2).unwrap().as_str().parse::<u32>().unwrap_or(0);
                
                if let Some(table) = tables.iter_mut().find(|t| t.name == table_name) {
                    table.status = TableStatus::InProgress;
                    table.progress_percent = Some(progress);
                    if table.started_at.is_none() {
                        table.started_at = Some(Utc::now());
                    }
                }
            }

            // Check for completed tables
            if let Some(caps) = completed_pattern.captures(line) {
                let table_name = caps.get(1).unwrap().as_str();
                if let Some(table) = tables.iter_mut().find(|t| t.name == table_name) {
                    table.status = TableStatus::Completed;
                    table.progress_percent = Some(100);
                    table.completed_at = Some(Utc::now());
                }
            }

            // Check for errors
            if let Some(caps) = error_pattern.captures(line) {
                let table_name = caps.get(1).unwrap().as_str();
                if let Some(table) = tables.iter_mut().find(|t| t.name == table_name) {
                    table.status = TableStatus::Error;
                    table.error_message = Some("Error during backup".to_string());
                }
            }
        }

        Ok(tables)
    }
}
