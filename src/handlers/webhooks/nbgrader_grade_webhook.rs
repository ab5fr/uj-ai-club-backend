use axum::{Json, extract::State};

use crate::{AppState, error::AppError, models::*};

/// Webhook endpoint for nbgrader to report grades
pub async fn nbgrader_grade_webhook(
    State(state): State<AppState>,
    Json(payload): Json<NbgraderWebhookPayload>,
) -> Result<Json<NbgraderWebhookResponse>, AppError> {
    // Verify webhook secret (allow empty for development)
    let expected_secret = std::env::var("NBGRADER_WEBHOOK_SECRET").unwrap_or_default();

    // Only verify if secret is configured (non-empty)
    if !expected_secret.is_empty() && payload.webhook_secret != expected_secret {
        return Err(AppError::AuthError);
    }

    // Find the notebook by assignment name
    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE assignment_name = $1")
            .bind(&payload.assignment_name)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound)?;

    // Find the user by jupyterhub username
    let user: User = sqlx::query_as("SELECT * FROM users WHERE jupyterhub_username = $1")
        .bind(&payload.student_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound)?;

    // Update latest in-progress or pending submission metadata; keep it pending for manual grading
    let _submission: ChallengeSubmission = sqlx::query_as(
        r#"
        UPDATE challenge_submissions
        SET status = 'grading_pending',
            score = $1, 
            max_score = $2, 
            nbgrader_submission_id = $4,
            submitted_at = COALESCE(submitted_at, NOW()),
            updated_at = NOW()
        WHERE id = (
            SELECT id
            FROM challenge_submissions
            WHERE user_id = $5 AND challenge_id = $6
              AND status IN ('in_progress', 'grading_pending')
            ORDER BY attempt_number DESC
            LIMIT 1
        )
        RETURNING *
        "#,
    )
    .bind(payload.score)
    .bind(payload.max_score)
    .bind(&payload.submission_id)
    .bind(user.id)
    .bind(notebook.challenge_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound)?;

    Ok(Json(NbgraderWebhookResponse {
        success: true,
        points_awarded: 0,
        message: format!(
            "Submission received for {} and marked as grading_pending",
            notebook.assignment_name
        ),
    }))
}
