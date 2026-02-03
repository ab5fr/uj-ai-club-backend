use axum::{Json, extract::State};

use crate::{AppState, error::AppError, models::*};

use super::update_user_ranks::update_user_ranks;

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

    // Calculate points to award (proportional to max_points)
    let score_percentage = if payload.max_score > 0.0 {
        payload.score / payload.max_score
    } else {
        0.0
    };
    let points_awarded = (score_percentage * notebook.max_points as f64).round() as i32;

    // Update the submission
    let submission: ChallengeSubmission = sqlx::query_as(
        r#"
        UPDATE challenge_submissions 
        SET status = 'graded', 
            score = $1, 
            max_score = $2, 
            points_awarded = $3,
            nbgrader_submission_id = $4,
            submitted_at = COALESCE(submitted_at, NOW()),
            graded_at = NOW(),
            updated_at = NOW()
        WHERE user_id = $5 AND challenge_id = $6
        RETURNING *
        "#,
    )
    .bind(payload.score)
    .bind(payload.max_score)
    .bind(points_awarded)
    .bind(&payload.submission_id)
    .bind(user.id)
    .bind(notebook.challenge_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound)?;

    // Credit points to user if not already credited
    if !submission.points_credited {
        sqlx::query("UPDATE users SET points = points + $1 WHERE id = $2")
            .bind(points_awarded)
            .bind(user.id)
            .execute(&state.pool)
            .await?;

        // Mark as credited
        sqlx::query("UPDATE challenge_submissions SET points_credited = true WHERE id = $1")
            .bind(submission.id)
            .execute(&state.pool)
            .await?;

        // Update user ranks
        update_user_ranks(&state.pool).await?;
    }

    Ok(Json(NbgraderWebhookResponse {
        success: true,
        points_awarded,
        message: format!(
            "Graded successfully: {}/{} points",
            points_awarded, notebook.max_points
        ),
    }))
}
