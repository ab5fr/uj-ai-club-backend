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

/// Submit a challenge - marks submission as submitted and triggers grading
/// This endpoint is called from the frontend when the user clicks "Submit"
pub async fn submit_challenge(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(challenge_id): Path<i32>,
) -> Result<Json<SubmitChallengeResponse>, AppError> {
    let challenge: Challenge = sqlx::query_as("SELECT * FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let allowed_submissions = challenge.allowed_submissions.max(1);

    // Get the notebook info
    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("This challenge does not have a notebook".to_string())
            })?;

    let attempts_used: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM challenge_submissions WHERE user_id = $1 AND challenge_id = $2",
    )
    .bind(auth.user_id)
    .bind(challenge_id)
    .fetch_one(&state.pool)
    .await?;

    let attempts_remaining = (allowed_submissions as i64 - attempts_used).max(0);

    // Get latest in-progress attempt
    let submission: Option<ChallengeSubmission> = sqlx::query_as(
        r#"
        SELECT * FROM challenge_submissions
        WHERE user_id = $1 AND challenge_id = $2 AND status = 'in_progress'
        ORDER BY attempt_number DESC
        LIMIT 1
        "#,
    )
    .bind(auth.user_id)
    .bind(challenge_id)
    .fetch_optional(&state.pool)
    .await?;

    let submission = if let Some(submission) = submission {
        submission
    } else {
        let latest_submission: Option<ChallengeSubmission> = sqlx::query_as(
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

        if let Some(latest) = latest_submission {
            if latest.status == "grading_pending" {
                return Ok(Json(SubmitChallengeResponse {
                    success: true,
                    message: "Your submission is pending manual grading by an admin.".to_string(),
                    status: "grading_pending".to_string(),
                    attempt_number: latest.attempt_number,
                    attempts_used,
                    attempts_remaining,
                }));
            }
        }

        return Err(AppError::BadRequest(
            "No in-progress attempt found. Start the challenge before submitting.".to_string(),
        ));
    };

    // Get the user's JupyterHub username
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.pool)
        .await?;

    let jupyterhub_username = user
        .jupyterhub_username
        .ok_or_else(|| AppError::BadRequest("JupyterHub username not set".to_string()))?;

    // Update submission status to "grading_pending"
    sqlx::query(
        r#"
        UPDATE challenge_submissions
        SET status = 'grading_pending',
            submitted_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(submission.id)
    .execute(&state.pool)
    .await?
    ;

    // Call JupyterHub API to trigger submission/grading
    // The grading service will watch for the submission and grade it
    let grading_service_url = std::env::var("GRADING_SERVICE_URL")
        .unwrap_or_else(|_| "http://uj-ai-club-grading:9100".to_string());

    // Call grading service API to trigger submission/grading
    // This copies the notebook from user's workspace to nbgrader exchange and grades it
    let client = reqwest::Client::new();
    let trigger_url = format!(
        "{}/submit/{}/{}",
        grading_service_url, jupyterhub_username, notebook.assignment_name
    );

    // Include the notebook filename and path in the request
    let payload = serde_json::json!({
        "notebookFilename": notebook.notebook_filename,
        "notebookPath": notebook.notebook_path
    });

    match client.post(&trigger_url).json(&payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                tracing::info!(
                    "Grading triggered for user {} on assignment {}",
                    jupyterhub_username,
                    notebook.assignment_name
                );
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                tracing::warn!("Failed to trigger grading: {} - {}", status, error_text);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to call grading service: {}", e);
        }
    }

    Ok(Json(SubmitChallengeResponse {
        success: true,
        message: "Submission received and marked as grading pending. An admin will review it manually.".to_string(),
        status: "grading_pending".to_string(),
        attempt_number: submission.attempt_number,
        attempts_used,
        attempts_remaining,
    }))
}
