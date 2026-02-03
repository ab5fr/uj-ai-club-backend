use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

/// Get notebook for a specific challenge (admin)
pub async fn admin_get_notebook_by_challenge(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(challenge_id): Path<i32>,
) -> Result<Json<AdminItemResponse<AdminChallengeNotebookResponse>>, AppError> {
    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    let response = AdminChallengeNotebookResponse {
        id: notebook.id,
        challenge_id: notebook.challenge_id,
        assignment_name: notebook.assignment_name,
        notebook_filename: notebook.notebook_filename,
        notebook_path: notebook.notebook_path,
        max_points: notebook.max_points,
        cpu_limit: notebook.cpu_limit,
        memory_limit: notebook.memory_limit,
        time_limit_minutes: notebook.time_limit_minutes,
        network_disabled: notebook.network_disabled,
        created_at: notebook.created_at,
        updated_at: notebook.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
