use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{DatabaseConfig, CreateDatabaseConfigRequest, UpdateDatabaseConfigRequest};
use super::{ApiError, ApiResult, success_response, paginated_response};

#[derive(Deserialize)]
pub struct ListQuery {
    page: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(list_database_configs).post(create_database_config))
        .route("/:id", get(get_database_config).put(update_database_config).delete(delete_database_config))
        .route("/:id/test", post(test_database_connection))
        .route("/:id/permissions", get(check_database_permissions))
        .with_state(pool)
}

async fn list_database_configs(
    State(pool): State<SqlitePool>,
    Query(query): Query<ListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    let offset = (page - 1) * limit;

    let mut sql = "SELECT * FROM database_configs".to_string();
    let mut count_sql = "SELECT COUNT(*) as count FROM database_configs".to_string();
    
    if let Some(search) = &query.search {
        let search_clause = format!(" WHERE name LIKE '%{}%' OR host LIKE '%{}%' OR database_name LIKE '%{}%'", search, search, search);
        sql.push_str(&search_clause);
        count_sql.push_str(&search_clause);
    }
    
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));

    let configs: Vec<DatabaseConfig> = sqlx::query_as(&sql)
        .fetch_all(&pool)
        .await?;

    let total: (i64,) = sqlx::query_as(&count_sql)
        .fetch_one(&pool)
        .await?;

    Ok(paginated_response(configs, page, limit, total.0 as u64))
}

async fn get_database_config(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let config: Option<DatabaseConfig> = sqlx::query_as(
        "SELECT * FROM database_configs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    match config {
        Some(config) => Ok(success_response(config)),
        None => Err(ApiError::NotFound("Database configuration not found".to_string())),
    }
}

async fn create_database_config(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateDatabaseConfigRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Check if name already exists
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM database_configs WHERE name = ?"
    )
    .bind(&req.name)
    .fetch_optional(&pool)
    .await?;

    if existing.is_some() {
        return Err(ApiError::BadRequest("Database configuration with this name already exists".to_string()));
    }

    let config = DatabaseConfig::new(req);

    sqlx::query(
        r#"
        INSERT INTO database_configs (id, name, host, port, username, password, database_name, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&config.id)
    .bind(&config.name)
    .bind(&config.host)
    .bind(&config.port)
    .bind(&config.username)
    .bind(&config.password)
    .bind(&config.database_name)
    .bind(&config.created_at)
    .bind(&config.updated_at)
    .execute(&pool)
    .await?;

    Ok(success_response(config))
}

async fn update_database_config(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDatabaseConfigRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let mut config: DatabaseConfig = sqlx::query_as(
        "SELECT * FROM database_configs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Database configuration not found".to_string()))?;

    // Check if new name conflicts with existing config
    if let Some(ref new_name) = req.name {
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM database_configs WHERE name = ? AND id != ?"
        )
        .bind(new_name)
        .bind(&id)
        .fetch_optional(&pool)
        .await?;

        if existing.is_some() {
            return Err(ApiError::BadRequest("Database configuration with this name already exists".to_string()));
        }
    }

    config.update(req);

    sqlx::query(
        r#"
        UPDATE database_configs 
        SET name = ?, host = ?, port = ?, username = ?, password = ?, database_name = ?, updated_at = ?
        WHERE id = ?
        "#
    )
    .bind(&config.name)
    .bind(&config.host)
    .bind(&config.port)
    .bind(&config.username)
    .bind(&config.password)
    .bind(&config.database_name)
    .bind(&config.updated_at)
    .bind(&config.id)
    .execute(&pool)
    .await?;

    Ok(success_response(config))
}

async fn delete_database_config(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let result = sqlx::query("DELETE FROM database_configs WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Database configuration not found".to_string()));
    }

    Ok(success_response(serde_json::json!({"message": "Database configuration deleted successfully"})))
}

async fn test_database_connection(
    State(_pool): State<SqlitePool>,
    Path(_id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // TODO: Implement actual connection testing using mydumper or mysql client
    // For now, return a mock response
    Ok(success_response(serde_json::json!({
        "success": true,
        "message": "Connection test successful",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn check_database_permissions(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // Get database config
    let config: DatabaseConfig = sqlx::query_as(
        "SELECT * FROM database_configs WHERE id = ?"
    )
    .bind(&id)
    .fetch_one(&pool)
    .await?;

    // Test connection and check permissions
    let connection_string = format!(
        "mysql://{}:{}@{}:{}/{}",
        config.username,
        config.password,
        config.host,
        config.port,
        config.database_name
    );

    let pool = sqlx::MySqlPool::connect(&connection_string).await
        .map_err(|e| ApiError::InternalError(format!("Failed to connect to database: {}", e)))?;

    // Test if user can create databases by actually trying to create a test database
    let test_db_name = format!("rdumper_test_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string());
    let can_create_db = sqlx::query(&format!("CREATE DATABASE IF NOT EXISTS `{}`", test_db_name))
        .execute(&pool)
        .await
        .is_ok();
    
    // Clean up test database if we created it
    if can_create_db {
        let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS `{}`", test_db_name))
            .execute(&pool)
            .await;
    }

    // Test if user can create tables by trying to create a test table
    let test_table_name = format!("rdumper_test_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string());
    let can_create_tables = sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS `{}`.`{}` (id INT PRIMARY KEY)", 
        config.database_name, test_table_name
    ))
    .execute(&pool)
    .await
    .is_ok();
    
    // Clean up test table if we created it
    if can_create_tables {
        let _ = sqlx::query(&format!("DROP TABLE IF EXISTS `{}`.`{}`", config.database_name, test_table_name))
            .execute(&pool)
            .await;
    }

    // Get list of existing databases
    let databases: Vec<String> = sqlx::query_scalar("SHOW DATABASES")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    Ok(success_response(serde_json::json!({
        "can_create_databases": can_create_db,
        "can_create_tables": can_create_tables,
        "existing_databases": databases,
        "current_database": config.database_name,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}