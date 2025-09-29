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

    /// Parse table progress from mydumper log using thread tracking
    async fn parse_table_progress(&self, log_content: &str, table_names: &[String]) -> Result<Vec<TableProgress>> {
        let mut tables = Vec::new();
        
        // Regex patterns for actual mydumper log format
        // Format: 2025-09-29 14:53:21 [INFO] - Thread 3: `sbtest`.`sbtest3` [ 0% ] | Tables: 10/16
        let data_pattern = Regex::new(r"Thread (\d+): `[^`]+`\.`([^`]+)` \[ (\d+)% \]")?;
        let error_pattern = Regex::new(r"ERROR.*`([^`]+)`")?;
        let table_info_pattern = Regex::new(r"([^.]+)\.([^ ]+) has ~(\d+) rows")?;
        let finished_pattern = Regex::new(r"Finished dump at:")?;

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

        // Track which thread is working on which table
        let mut thread_to_table: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        let mut table_to_threads: std::collections::HashMap<String, std::collections::HashSet<u32>> = std::collections::HashMap::new();

        // Check if backup is finished
        let is_finished = finished_pattern.is_match(log_content);
        
        // Parse log lines
        for line in log_content.lines() {
            // Check for table info (table started)
            if let Some(caps) = table_info_pattern.captures(line) {
                let table_name = caps.get(2).unwrap().as_str(); // Second capture group is table name
                if let Some(table) = tables.iter_mut().find(|t| t.name == table_name) {
                    table.status = TableStatus::InProgress;
                    if table.started_at.is_none() {
                        table.started_at = Some(Utc::now());
                    }
                }
            }

            // Check for data progress and track thread assignments
            if let Some(caps) = data_pattern.captures(line) {
                let thread_id = caps.get(1).unwrap().as_str().parse::<u32>().unwrap_or(0);
                let table_name = caps.get(2).unwrap().as_str();
                let progress = caps.get(3).unwrap().as_str().parse::<u32>().unwrap_or(0);
                
                // Check if this thread was working on a different table before
                if let Some(previous_table) = thread_to_table.get(&thread_id) {
                    if previous_table != table_name {
                        // Thread switched to a new table, mark previous table as completed if no other threads are working on it
                        if let Some(threads) = table_to_threads.get(previous_table) {
                            if threads.len() <= 1 {
                                // Only this thread was working on the previous table, mark it as completed
                                if let Some(table) = tables.iter_mut().find(|t| t.name == *previous_table) {
                                    if !matches!(table.status, TableStatus::Error) {
                                        table.status = TableStatus::Completed;
                                        table.progress_percent = Some(100);
                                        table.completed_at = Some(Utc::now());
                                    }
                                }
                            }
                            // Remove this thread from the previous table's thread set
                            table_to_threads.get_mut(previous_table).unwrap().remove(&thread_id);
                        }
                    }
                }
                
                // Update thread-to-table mapping
                thread_to_table.insert(thread_id, table_name.to_string());
                
                // Update table-to-threads mapping
                table_to_threads.entry(table_name.to_string())
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(thread_id);
                
                // Update table progress
                if let Some(table) = tables.iter_mut().find(|t| t.name == table_name) {
                    table.status = TableStatus::InProgress;
                    table.progress_percent = Some(progress);
                    if table.started_at.is_none() {
                        table.started_at = Some(Utc::now());
                    }
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
        
        // If backup is finished, mark all remaining tables as completed
        if is_finished {
            for table in tables.iter_mut() {
                if !matches!(table.status, TableStatus::Error) && !matches!(table.status, TableStatus::Completed) {
                    table.status = TableStatus::Completed;
                    table.progress_percent = Some(100);
                    table.completed_at = Some(Utc::now());
                }
            }
        }

        Ok(tables)
    }
}
