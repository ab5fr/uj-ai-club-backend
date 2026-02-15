use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

/// Get user's submission for a specific challenge
pub async fn get_user_submission(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(challenge_id): Path<i32>,
) -> Result<Json<Option<UserSubmissionResponse>>, AppError> {
    let challenge: Challenge = sqlx::query_as("SELECT * FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let allowed_submissions = challenge.allowed_submissions.max(1);

    let attempts_used: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM challenge_submissions WHERE user_id = $1 AND challenge_id = $2",
    )
    .bind(auth.user_id)
    .bind(challenge_id)
    .fetch_one(&state.pool)
    .await?;

    let attempts_remaining = (allowed_submissions as i64 - attempts_used).max(0);

    let submission: Option<ChallengeSubmission> = sqlx::query_as(
        r#"
        SELECT * FROM challenge_submissions
        WHERE user_id = $1 AND challenge_id = $2
        ORDER BY attempt_number DESC
        LIMIT 1
        "#,
    )
    .bind(auth.user_id)
    .bind(challenge_id)
    .fetch_optional(&state.pool)
    .await?;

    let response = submission.map(|s| UserSubmissionResponse {
        id: s.id,
        challenge_id: s.challenge_id,
        attempt_number: s.attempt_number,
        status: s.status,
        score: s.score,
        max_score: s.max_score,
        points_awarded: s.points_awarded,
        started_at: s.started_at,
        submitted_at: s.submitted_at,
        graded_at: s.graded_at,
        allowed_submissions,
        attempts_used,
        attempts_remaining,
    });

    Ok(Json(response))
}
