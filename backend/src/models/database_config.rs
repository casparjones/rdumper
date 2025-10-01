use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DatabaseConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub database_name: String, // Database name (can be empty for connection-only configs)
    pub connection_status: String, // "untested", "success", "failed"
    pub last_tested: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDatabaseConfigRequest {
    pub name: String,
    pub host: String,
    pub port: Option<i32>,
    pub username: String,
    pub password: String,
    pub database_name: Option<String>, // Optional database name
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDatabaseConfigRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database_name: Option<String>,
}

impl DatabaseConfig {
    pub fn new(req: CreateDatabaseConfigRequest) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            host: req.host,
            port: req.port.unwrap_or(3306),
            username: req.username,
            password: req.password,
            database_name: req.database_name.unwrap_or_default(),
            connection_status: "untested".to_string(),
            last_tested: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update(&mut self, req: UpdateDatabaseConfigRequest) {
        if let Some(name) = req.name {
            self.name = name;
        }
        if let Some(host) = req.host {
            self.host = host;
        }
        if let Some(port) = req.port {
            self.port = port;
        }
        if let Some(username) = req.username {
            self.username = username;
        }
        if let Some(password) = req.password {
            self.password = password;
        }
        if let Some(database_name) = req.database_name {
            self.database_name = database_name;
        }
        // Reset connection status when config changes
        self.connection_status = "untested".to_string();
        self.last_tested = None;
        self.updated_at = Utc::now();
    }

    pub fn mark_connection_tested(&mut self, success: bool) {
        self.connection_status = if success { "success".to_string() } else { "failed".to_string() };
        self.last_tested = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn connection_string(&self) -> String {
        if self.database_name.is_empty() {
            format!(
                "mysql://{}:{}@{}:{}",
                self.username, self.password, self.host, self.port
            )
        } else {
            format!(
                "mysql://{}:{}@{}:{}/{}",
                self.username, self.password, self.host, self.port, self.database_name
            )
        }
    }

    pub fn connection_string_with_db(&self, db_name: &str) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, db_name
        )
    }

    pub fn get_database_name(&self) -> Option<&String> {
        if self.database_name.is_empty() {
            None
        } else {
            Some(&self.database_name)
        }
    }

}