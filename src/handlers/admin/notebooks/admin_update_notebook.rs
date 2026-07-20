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

/// Update notebook settings (admin)
pub async fn admin_update_notebook(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<i32>,
    Json(req): Json<AdminUpdateNotebookRequest>,
) -> Result<Json<AdminItemResponse<AdminChallengeNotebookResponse>>, AppError> {
    let existing: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE id = $1")
            .bind(notebook_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    let max_points = req.max_points.unwrap_or(existing.max_points);
    let cpu_limit = req.cpu_limit.unwrap_or(existing.cpu_limit);
    let memory_limit = req.memory_limit.unwrap_or(existing.memory_limit);
    let time_limit_minutes = req
        .time_limit_minutes
        .unwrap_or(existing.time_limit_minutes);
    let network_disabled = req.network_disabled.unwrap_or(existing.network_disabled);
    let auto_grade_enabled = req.auto_grade_enabled.unwrap_or(existing.auto_grade_enabled);

    let notebook: ChallengeNotebook = sqlx::query_as(
        r#"
        UPDATE challenge_notebooks 
        SET max_points = $1, cpu_limit = $2, memory_limit = $3, 
            time_limit_minutes = $4, network_disabled = $5, auto_grade_enabled = $6, updated_at = NOW()
        WHERE id = $7
        RETURNING *
        "#,
    )
    .bind(max_points)
    .bind(cpu_limit)
    .bind(&memory_limit)
    .bind(time_limit_minutes)
    .bind(network_disabled)
    .bind(auto_grade_enabled)
    .bind(notebook_id)
    .fetch_one(&state.pool)
    .await?;

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
        auto_grade_enabled: notebook.auto_grade_enabled,
        created_at: notebook.created_at,
        updated_at: notebook.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
