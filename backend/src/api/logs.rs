use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::models::{Log, LogType, LogLevel};
use super::{ApiError, ApiResult, success_response, paginated_response};

#[derive(Deserialize)]
pub struct ListLogsQuery {
    page: Option<u32>,
    limit: Option<u32>,
    log_type: Option<String>,
    entity_type: Option<String>,
    entity_id: Option<String>,
    level: Option<String>,
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(list_logs))
        .route("/cleanup", get(cleanup_logs))
        .with_state(pool)
}

async fn list_logs(
    State(pool): State<SqlitePool>,
    Query(query): Query<ListLogsQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(50);
    let offset = (page - 1) * limit;

    let mut sql = "SELECT * FROM logs WHERE 1=1".to_string();
    let mut count_sql = "SELECT COUNT(*) as count FROM logs WHERE 1=1".to_string();
    
    if let Some(log_type) = &query.log_type {
        let log_type_clause = format!(" AND log_type = '{}'", log_type);
        sql.push_str(&log_type_clause);
        count_sql.push_str(&log_type_clause);
    }
    
    if let Some(entity_type) = &query.entity_type {
        let entity_type_clause = format!(" AND entity_type = '{}'", entity_type);
        sql.push_str(&entity_type_clause);
        count_sql.push_str(&entity_type_clause);
    }
    
    if let Some(entity_id) = &query.entity_id {
        let entity_id_clause = format!(" AND entity_id = '{}'", entity_id);
        sql.push_str(&entity_id_clause);
        count_sql.push_str(&entity_id_clause);
    }
    
    if let Some(level) = &query.level {
        let level_clause = format!(" AND level = '{}'", level);
        sql.push_str(&level_clause);
        count_sql.push_str(&level_clause);
    }
    
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));

    let logs: Vec<Log> = sqlx::query_as(&sql)
        .fetch_all(&pool)
        .await?;

    let total: (i64,) = sqlx::query_as(&count_sql)
        .fetch_one(&pool)
        .await?;

    Ok(paginated_response(logs, page, limit, total.0 as u64))
}

async fn cleanup_logs(
    State(pool): State<SqlitePool>,
    Query(params): Query<serde_json::Value>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let days = params.get("days")
        .and_then(|v| v.as_u64())
        .unwrap_or(14) as u32;

    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days as i64);
    
    let result = sqlx::query("DELETE FROM logs WHERE created_at < ?")
        .bind(cutoff_date)
        .execute(&pool)
        .await?;

    Ok(success_response(serde_json::json!({
        "message": format!("Cleaned up {} old log entries", result.rows_affected()),
        "deleted_count": result.rows_affected(),
        "cutoff_date": cutoff_date.to_rfc3339()
    })))
}
