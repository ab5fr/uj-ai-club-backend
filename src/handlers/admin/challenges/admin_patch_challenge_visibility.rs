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

pub async fn admin_patch_challenge_visibility(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<AdminVisibilityRequest>,
) -> Result<Json<AdminItemResponse<AdminChallengeResponse>>, AppError> {
    let challenge: Challenge = sqlx::query_as(
        "UPDATE challenges SET visible = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
    )
    .bind(req.visible)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let response = AdminChallengeResponse {
        id: challenge.id,
        title: challenge.title,
        description: challenge.description,
        start_date: challenge.start_date,
        end_date: challenge.end_date,
        visible: challenge.visible,
        created_at: challenge.created_at,
        updated_at: challenge.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
