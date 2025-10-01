use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    #[serde(rename = "backup")]
    Backup,
    #[serde(rename = "restore")]
    Restore,
    #[serde(rename = "cleanup")]
    Cleanup,
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobType::Backup => write!(f, "backup"),
            JobType::Restore => write!(f, "restore"),
            JobType::Cleanup => write!(f, "cleanup"),
        }
    }
}

impl std::str::FromStr for JobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "backup" => Ok(JobType::Backup),
            "restore" => Ok(JobType::Restore),
            "cleanup" => Ok(JobType::Cleanup),
            _ => Err(format!("Invalid job type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

impl Default for JobStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(JobStatus::Pending),
            "running" => Ok(JobStatus::Running),
            "completed" => Ok(JobStatus::Completed),
            "failed" => Ok(JobStatus::Failed),
            "cancelled" => Ok(JobStatus::Cancelled),
            _ => Err(format!("Invalid job status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Job {
    pub id: String,
    pub task_id: Option<String>,
    pub used_database: Option<String>,
    pub job_type: String,
    pub status: String,
    pub progress: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub log_output: Option<String>,
    pub backup_path: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub task_id: Option<String>,
    pub used_database: Option<String>,
    pub job_type: JobType,
    pub backup_path: Option<String>,
}

impl Job {
    pub fn new(req: CreateJobRequest) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            task_id: req.task_id,
            used_database: req.used_database,
            job_type: req.job_type.to_string(),
            status: JobStatus::default().to_string(),
            progress: 0,
            started_at: None,
            completed_at: None,
            error_message: None,
            log_output: None,
            backup_path: req.backup_path,
            created_at: now,
        }
    }

    // pub fn start(&mut self) {
    //     self.status = JobStatus::Running.to_string();
    //     self.started_at = Some(Utc::now());
    // }

    // pub fn complete(&mut self) {
    //     self.status = JobStatus::Completed.to_string();
    //     self.completed_at = Some(Utc::now());
    //     self.progress = 100;
    // }

    // pub fn fail(&mut self, error: String) {
    //     self.status = JobStatus::Failed.to_string();
    //     self.completed_at = Some(Utc::now());
    //     self.error_message = Some(error);
    // }

    // pub fn update_progress(&mut self, progress: i32) {
    //     self.progress = progress.clamp(0, 100);
    // }

    // pub fn append_log(&mut self, log: &str) {
    //     match &self.log_output {
    //         Some(existing) => {
    //             self.log_output = Some(format!("{}\n{}", existing, log));
    //         }
    //         None => {
    //             self.log_output = Some(log.to_string());
    //         }
    //     }
    // }

    // pub fn job_type(&self) -> Result<JobType, String> {
    //     self.job_type.parse()
    // }

    pub fn status(&self) -> Result<JobStatus, String> {
        self.status.parse()
    }

    // pub fn duration(&self) -> Option<chrono::Duration> {
    //     match (self.started_at, self.completed_at) {
    //         (Some(start), Some(end)) => Some(end - start),
    //         _ => None,
    //     }
    // }
}