use chrono::{DateTime, Utc, Duration, Timelike, Datelike};
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
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
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
            last_run: None,
            next_run: None, // Will be calculated when task is saved
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

    /// Calculate the next run time based on the cron schedule
    pub fn calculate_next_run(&self) -> Result<Option<DateTime<Utc>>, String> {
        if !self.is_active {
            return Ok(None);
        }

        // Simple cron parser for common patterns
        let next_run = self.parse_cron_schedule(&self.cron_schedule)?;
        Ok(Some(next_run))
    }

    /// Simple cron parser for common patterns
    fn parse_cron_schedule(&self, cron_expr: &str) -> Result<DateTime<Utc>, String> {
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(format!("Invalid cron format. Expected 5 parts, got {}", parts.len()));
        }

        let now = Utc::now();
        
        // Handle common patterns
        match cron_expr {
            "* * * * *" => {
                // Every minute - next minute
                Ok(now + Duration::minutes(1))
            },
            "0 * * * *" => {
                // Every hour at minute 0
                let next_hour = now + Duration::hours(1);
                Ok(DateTime::from_timestamp(next_hour.timestamp(), 0)
                    .unwrap_or(next_hour)
                    .with_minute(0)
                    .unwrap_or(next_hour)
                    .with_second(0)
                    .unwrap_or(next_hour)
                    .with_nanosecond(0)
                    .unwrap_or(next_hour))
            },
            "0 0 * * *" => {
                // Daily at midnight
                let tomorrow = now + Duration::days(1);
                Ok(DateTime::from_timestamp(tomorrow.timestamp(), 0)
                    .unwrap_or(tomorrow)
                    .with_hour(0)
                    .unwrap_or(tomorrow)
                    .with_minute(0)
                    .unwrap_or(tomorrow)
                    .with_second(0)
                    .unwrap_or(tomorrow)
                    .with_nanosecond(0)
                    .unwrap_or(tomorrow))
            },
            "0 0 * * 1" => {
                // Weekly on Monday at midnight
                let days_until_monday = (8 - now.weekday().num_days_from_monday()) % 7;
                let next_monday = if days_until_monday == 0 {
                    now + Duration::days(7) // Next Monday if today is Monday
                } else {
                    now + Duration::days(days_until_monday as i64)
                };
                Ok(DateTime::from_timestamp(next_monday.timestamp(), 0)
                    .unwrap_or(next_monday)
                    .with_hour(0)
                    .unwrap_or(next_monday)
                    .with_minute(0)
                    .unwrap_or(next_monday)
                    .with_second(0)
                    .unwrap_or(next_monday)
                    .with_nanosecond(0)
                    .unwrap_or(next_monday))
            },
            _ => {
                // Try to parse as specific time pattern (minute hour * * *)
                if let Some(next_run) = self.parse_specific_time_pattern(&parts, now) {
                    Ok(next_run)
                } else if let Some(interval) = self.parse_interval_pattern(cron_expr) {
                    Ok(now + interval)
                } else {
                    Err(format!("Unsupported cron pattern: {}", cron_expr))
                }
            }
        }
    }

    /// Parse specific time patterns like "0 1 * * *" (daily at 1:00 AM)
    fn parse_specific_time_pattern(&self, parts: &[&str], now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Pattern: minute hour * * *
        if parts[2] == "*" && parts[3] == "*" && parts[4] == "*" {
            if let (Ok(minute), Ok(hour)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if minute <= 59 && hour <= 23 {
                    // Calculate next occurrence of this time
                    let mut next_run = now
                        .with_hour(hour)
                        .unwrap_or(now)
                        .with_minute(minute)
                        .unwrap_or(now)
                        .with_second(0)
                        .unwrap_or(now)
                        .with_nanosecond(0)
                        .unwrap_or(now);

                    // If the time has already passed today, schedule for tomorrow
                    if next_run <= now {
                        next_run = next_run + Duration::days(1);
                    }

                    return Some(next_run);
                }
            }
        }
        None
    }

    /// Parse interval patterns like "*/5 * * * *" (every 5 minutes)
    fn parse_interval_pattern(&self, cron_expr: &str) -> Option<Duration> {
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        if parts.len() != 5 {
            return None;
        }

        // Check for interval patterns in minutes
        if parts[0].starts_with("*/") && parts[1] == "*" && parts[2] == "*" && parts[3] == "*" && parts[4] == "*" {
            if let Ok(minutes) = parts[0][2..].parse::<i64>() {
                return Some(Duration::minutes(minutes));
            }
        }

        // Check for interval patterns in hours
        if parts[0] == "0" && parts[1].starts_with("*/") && parts[2] == "*" && parts[3] == "*" && parts[4] == "*" {
            if let Ok(hours) = parts[1][2..].parse::<i64>() {
                return Some(Duration::hours(hours));
            }
        }

        None
    }

    /// Update the next run time based on current cron schedule
    pub fn update_next_run(&mut self) -> Result<(), String> {
        self.next_run = self.calculate_next_run()?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark task as executed and calculate next run
    pub fn mark_executed(&mut self) -> Result<(), String> {
        self.last_run = Some(Utc::now());
        self.update_next_run()?;
        Ok(())
    }

    /// Check if the task should run now
    pub fn should_run_now(&self) -> bool {
        if !self.is_active {
            return false;
        }

        match self.next_run {
            Some(next_run) => Utc::now() >= next_run,
            None => false,
        }
    }
}