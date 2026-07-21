use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
}

pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let db_ok = sqlx::query("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();

    if db_ok {
        Ok(Json(HealthResponse {
            status: "ok".to_string(),
        }))
    } else {
        tracing::error!("Health check failed: database unreachable");
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}
