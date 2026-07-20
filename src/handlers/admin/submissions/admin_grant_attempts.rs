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

pub async fn admin_grant_attempts(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(submission_id): Path<uuid::Uuid>,
    Json(req): Json<AdminGrantAttemptsRequest>,
) -> Result<Json<AdminSuccessResponse>, AppError> {
    if req.extra_attempts < 0 {
        return Err(AppError::BadRequest(
            "extraAttempts must be zero or greater".to_string(),
        ));
    }

    #[derive(sqlx::FromRow)]
    struct Target {
        user_id: uuid::Uuid,
        challenge_id: i32,
    }

    let target: Target = sqlx::query_as(
        "SELECT user_id, challenge_id FROM challenge_submissions WHERE id = $1",
    )
    .bind(submission_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    sqlx::query(
        r#"
        INSERT INTO challenge_attempt_overrides (user_id, challenge_id, extra_attempts, updated_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_id, challenge_id)
        DO UPDATE SET extra_attempts = challenge_attempt_overrides.extra_attempts + EXCLUDED.extra_attempts,
                      updated_at = NOW()
        "#,
    )
    .bind(target.user_id)
    .bind(target.challenge_id)
    .bind(req.extra_attempts)
    .execute(&state.pool)
    .await?;

    Ok(Json(AdminSuccessResponse { success: true }))
}
