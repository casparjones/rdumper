pub mod database_configs;
pub mod tasks;
pub mod jobs;
pub mod backups;
pub mod system;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::SqlitePool;

pub fn create_routes(pool: SqlitePool) -> Router {
    Router::new()
        .nest("/api/database-configs", database_configs::routes(pool.clone()))
        .nest("/api/tasks", tasks::routes(pool.clone()))
        .nest("/api/jobs", jobs::routes(pool.clone()))
        .nest("/api/backups", backups::routes(pool.clone()))
        .nest("/api/system", system::routes())
        .route("/api/health", get(health_check))
}

async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "rdumper-backend"
    }))
}

// Common error handling
#[derive(Debug)]
pub enum ApiError {
    DatabaseError(sqlx::Error),
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::DatabaseError(err)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ApiError::DatabaseError(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }));

        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

// Common response types
pub fn success_response<T: serde::Serialize>(data: T) -> impl IntoResponse {
    Json(json!({
        "success": true,
        "data": data,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub fn paginated_response<T: serde::Serialize>(
    data: Vec<T>,
    page: u32,
    limit: u32,
    total: u64,
) -> impl IntoResponse {
    let total_pages = (total as f64 / limit as f64).ceil() as u32;
    
    Json(json!({
        "success": true,
        "data": data,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "total_pages": total_pages,
            "has_next": page < total_pages,
            "has_prev": page > 1
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}