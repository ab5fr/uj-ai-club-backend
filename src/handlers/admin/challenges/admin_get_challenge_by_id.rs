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

pub async fn admin_get_challenge_by_id(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdminItemResponse<AdminChallengeResponse>>, AppError> {
    let challenge: Challenge = sqlx::query_as("SELECT * FROM challenges WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let response = AdminChallengeResponse {
        id: challenge.id,
        title: challenge.title,
        description: challenge.description,
        allowed_submissions: challenge.allowed_submissions,
        start_date: challenge.start_date,
        end_date: challenge.end_date,
        visible: challenge.visible,
        created_at: challenge.created_at,
        updated_at: challenge.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
