use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use crate::services::TaskWorker;

#[derive(Debug, Serialize)]
pub struct WorkerStatusResponse {
    pub is_running: bool,
    pub last_tick: Option<String>,
    pub total_ticks: u64,
    pub tasks_executed: u64,
    pub status_color: String,
    pub status_text: String,
}

impl From<crate::services::WorkerStatus> for WorkerStatusResponse {
    fn from(status: crate::services::WorkerStatus) -> Self {
        let now = chrono::Utc::now();
        let (status_color, status_text) = match status.last_tick {
            Some(last_tick) => {
                let duration = now - last_tick;
                if duration.num_seconds() <= 60 {
                    ("green".to_string(), "Running".to_string())
                } else {
                    ("red".to_string(), "Stale".to_string())
                }
            }
            None => ("gray".to_string(), "Not started".to_string()),
        };

        Self {
            is_running: status.is_running,
            last_tick: status.last_tick.map(|t| t.to_rfc3339()),
            total_ticks: status.total_ticks,
            tasks_executed: status.tasks_executed,
            status_color,
            status_text,
        }
    }
}

pub fn routes(worker: Arc<TaskWorker>) -> Router {
    Router::new()
        .route("/status", get(get_worker_status))
        .route("/start", post(start_worker))
        .with_state(worker)
}

async fn get_worker_status(
    State(worker): State<Arc<TaskWorker>>,
) -> crate::api::ApiResult<impl axum::response::IntoResponse> {
    let status = worker.get_status();
    let response = WorkerStatusResponse::from(status);
    Ok(crate::api::success_response(response))
}

async fn start_worker(
    State(worker): State<Arc<TaskWorker>>,
) -> crate::api::ApiResult<impl axum::response::IntoResponse> {
    let status = worker.get_status();
    
    if status.is_running {
        return Err(crate::api::ApiError::BadRequest(
            "Worker is already running".to_string()
        ));
    }

    // Note: In a real implementation, you might want to spawn the worker
    // in a separate task here. For now, we'll just return a message.
    Ok(crate::api::success_response(serde_json::json!({
        "message": "Worker start requested. Note: Worker should be started automatically on application startup.",
        "current_status": WorkerStatusResponse::from(status)
    })))
}
