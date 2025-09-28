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
    pub database_name: String,
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
    pub database_name: String,
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
            database_name: req.database_name,
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
        self.updated_at = Utc::now();
    }

    // pub fn connection_string(&self) -> String {
    //     format!(
    //         "mysql://{}:{}@{}:{}/{}",
    //         self.username, self.password, self.host, self.port, self.database_name
    //     )
    // }
}