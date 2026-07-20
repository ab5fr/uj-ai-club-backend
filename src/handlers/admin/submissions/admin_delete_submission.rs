use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
    submissions::sync_best_attempt_points,
};

pub async fn admin_delete_submission(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(submission_id): Path<uuid::Uuid>,
) -> Result<Json<AdminSuccessResponse>, AppError> {
    #[derive(sqlx::FromRow)]
    struct Target {
        user_id: uuid::Uuid,
        challenge_id: i32,
    }

    let target: Option<Target> = sqlx::query_as(
        "SELECT user_id, challenge_id FROM challenge_submissions WHERE id = $1",
    )
    .bind(submission_id)
    .fetch_optional(&state.pool)
    .await?;

    let Some(target) = target else {
        return Err(AppError::NotFound);
    };

    let deleted = sqlx::query("DELETE FROM challenge_submissions WHERE id = $1")
        .bind(submission_id)
        .execute(&state.pool)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    sync_best_attempt_points(&state.pool, target.user_id, target.challenge_id).await?;

    Ok(Json(AdminSuccessResponse { success: true }))
}
