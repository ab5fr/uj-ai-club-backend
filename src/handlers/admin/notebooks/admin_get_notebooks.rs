use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

/// Get all challenge notebooks (admin)
pub async fn admin_get_notebooks(
    _auth: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<AdminItemsResponse<AdminChallengeNotebookResponse>>, AppError> {
    let notebooks: Vec<ChallengeNotebook> =
        sqlx::query_as("SELECT * FROM challenge_notebooks ORDER BY id")
            .fetch_all(&state.pool)
            .await?;

    let responses: Vec<AdminChallengeNotebookResponse> = notebooks
        .into_iter()
        .map(|n| AdminChallengeNotebookResponse {
            id: n.id,
            challenge_id: n.challenge_id,
            assignment_name: n.assignment_name,
            notebook_filename: n.notebook_filename,
            notebook_path: n.notebook_path,
            max_points: n.max_points,
            cpu_limit: n.cpu_limit,
            memory_limit: n.memory_limit,
            time_limit_minutes: n.time_limit_minutes,
            network_disabled: n.network_disabled,
            created_at: n.created_at,
            updated_at: n.updated_at,
        })
        .collect();

    Ok(Json(AdminItemsResponse { items: responses }))
}
