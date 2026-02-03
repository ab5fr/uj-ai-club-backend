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
    // Get the user's submission
    let submission: ChallengeSubmission = sqlx::query_as(
        "SELECT * FROM challenge_submissions WHERE user_id = $1 AND challenge_id = $2",
    )
    .bind(auth.user_id)
    .bind(challenge_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("You haven't started this challenge yet".to_string()))?;

    // Check if already graded
    if submission.status == "graded" {
        return Ok(Json(SubmitChallengeResponse {
            success: true,
            message: "Challenge already graded".to_string(),
            status: "graded".to_string(),
        }));
    }

    // Get the notebook info
    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("This challenge does not have a notebook".to_string())
            })?;

    // Get the user's JupyterHub username
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.pool)
        .await?;

    let jupyterhub_username = user
        .jupyterhub_username
        .ok_or_else(|| AppError::BadRequest("JupyterHub username not set".to_string()))?;

    // Update submission status to "submitted"
    sqlx::query(
        r#"
        UPDATE challenge_submissions 
        SET status = 'submitted', 
            submitted_at = NOW(),
            updated_at = NOW()
        WHERE user_id = $1 AND challenge_id = $2
        "#,
    )
    .bind(auth.user_id)
    .bind(challenge_id)
    .execute(&state.pool)
    .await?;

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
        message: "Challenge submitted! Grading will begin shortly.".to_string(),
        status: "submitted".to_string(),
    }))
}
