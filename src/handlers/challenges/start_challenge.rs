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

/// Start a challenge - creates submission record and returns JupyterHub URL
pub async fn start_challenge(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(challenge_id): Path<i32>,
) -> Result<Json<StartChallengeResponse>, AppError> {
    // Verify the challenge exists and has a notebook
    let challenge: Challenge =
        sqlx::query_as("SELECT * FROM challenges WHERE id = $1 AND visible = true")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    // Check if challenge is within date range
    let now = time::OffsetDateTime::now_utc();
    if let Some(start_date) = challenge.start_date
        && now < start_date
    {
        return Err(AppError::BadRequest(
            "Challenge has not started yet".to_string(),
        ));
    }
    if let Some(end_date) = challenge.end_date
        && now > end_date
    {
        return Err(AppError::BadRequest("Challenge has ended".to_string()));
    }

    // Get the notebook for this challenge
    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("This challenge does not have a notebook".to_string())
            })?;

    // Get user info (verify user exists)
    let _user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    // Generate or get JupyterHub username for this user
    let jupyterhub_username = format!("user_{}", auth.user_id.to_string().replace("-", ""));

    // Update user's jupyterhub_username if not set
    sqlx::query(
        "UPDATE users SET jupyterhub_username = $1 WHERE id = $2 AND jupyterhub_username IS NULL",
    )
    .bind(&jupyterhub_username)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await?;

    // Check if submission already exists
    let existing_submission: Option<ChallengeSubmission> = sqlx::query_as(
        "SELECT * FROM challenge_submissions WHERE user_id = $1 AND challenge_id = $2",
    )
    .bind(auth.user_id)
    .bind(challenge_id)
    .fetch_optional(&state.pool)
    .await?;

    let submission_id = if let Some(existing) = existing_submission {
        // If already graded, don't allow restart
        if existing.status == "graded" {
            return Err(AppError::BadRequest(
                "You have already completed this challenge".to_string(),
            ));
        }
        existing.id
    } else {
        // Create new submission
        let new_submission: ChallengeSubmission = sqlx::query_as(
            r#"
            INSERT INTO challenge_submissions (user_id, challenge_id, notebook_id, status, started_at)
            VALUES ($1, $2, $3, 'in_progress', NOW())
            RETURNING *
            "#
        )
        .bind(auth.user_id)
        .bind(challenge_id)
        .bind(notebook.id)
        .fetch_one(&state.pool)
        .await?;

        // Update user stats
        sqlx::query(
            "UPDATE user_stats SET challenges_taken = challenges_taken + 1, updated_at = NOW() WHERE user_id = $1"
        )
        .bind(auth.user_id)
        .execute(&state.pool)
        .await?;

        new_submission.id
    };

    // Create a JWT token for JupyterHub SSO
    let jupyterhub_token =
        crate::auth::create_jupyterhub_token(auth.user_id, &jupyterhub_username)?;

    // Call grading service to prepare the notebook in user's workspace
    // This copies and processes the notebook (removes solutions) for the user
    let grading_service_url = std::env::var("GRADING_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:9100".to_string());

    let client = reqwest::Client::new();
    let prepare_url = format!(
        "{}/prepare-notebook/{}/{}",
        grading_service_url, jupyterhub_username, notebook.assignment_name
    );

    // Try to prepare the notebook, but don't fail if grading service is unavailable
    // The pre_spawn_hook might still copy it
    match client
        .post(&prepare_url)
        .json(&serde_json::json!({
            "notebookPath": notebook.notebook_path,
            "notebookFilename": notebook.notebook_filename
        }))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                tracing::info!("Notebook prepared for user {}", jupyterhub_username);
            } else {
                tracing::warn!(
                    "Failed to prepare notebook: {} - will rely on pre_spawn_hook",
                    resp.status()
                );
            }
        }
        Err(e) => {
            tracing::warn!("Could not reach grading service to prepare notebook: {}", e);
        }
    }

    // Generate JupyterHub URL
    // Use notebook_filename (the original filename) since the pre-spawn hook copies notebooks
    // with their original filename (stripping only the UUID prefix)
    let jupyterhub_base_url = std::env::var("JUPYTERHUB_URL")
        .unwrap_or_else(|_| "https://jupyter.aiclub-uj.com".to_string());

    // URL pattern: login -> spawn -> notebook
    // The spawn endpoint ensures the server is started before redirecting to the notebook
    let next_path = format!(
        "/user/{}/notebooks/{}",
        jupyterhub_username, notebook.notebook_filename
    );
    let encoded_next = urlencoding::encode(&next_path);
    let jupyterhub_url = format!(
        "{}/hub/login?token={}&next=/hub/spawn/{}?next={}",
        jupyterhub_base_url, jupyterhub_token, jupyterhub_username, encoded_next
    );

    Ok(Json(StartChallengeResponse {
        success: true,
        jupyterhub_url,
        submission_id,
        token: jupyterhub_token,
    }))
}
