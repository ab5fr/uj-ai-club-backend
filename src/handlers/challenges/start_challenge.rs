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

    let allowed_submissions = challenge.allowed_submissions.max(1);

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
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    // Generate or get JupyterHub username for this user
    let generated_username = format!("user_{}", auth.user_id.to_string().replace("-", ""));
    let jupyterhub_username = user
        .jupyterhub_username
        .clone()
        .unwrap_or(generated_username.clone());

    // Update user's jupyterhub_username if not set
    sqlx::query(
        "UPDATE users SET jupyterhub_username = $1 WHERE id = $2 AND jupyterhub_username IS NULL",
    )
    .bind(&jupyterhub_username)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await?;

    let attempts_used: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM challenge_submissions WHERE user_id = $1 AND challenge_id = $2",
    )
    .bind(auth.user_id)
    .bind(challenge_id)
    .fetch_one(&state.pool)
    .await?;

    // Reuse current in-progress attempt if it exists
    let existing_in_progress: Option<ChallengeSubmission> = sqlx::query_as(
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

    let (submission_id, attempt_number, attempts_used_after) = if let Some(existing) =
        existing_in_progress
    {
        (existing.id, existing.attempt_number, attempts_used)
    } else {
        if attempts_used >= allowed_submissions as i64 {
            return Err(AppError::BadRequest(format!(
                "Submission limit reached for this challenge ({} attempts)",
                allowed_submissions
            )));
        }

        let next_attempt_number = attempts_used as i32 + 1;

        // Create new attempt
        let new_submission: ChallengeSubmission = sqlx::query_as(
            r#"
            INSERT INTO challenge_submissions (user_id, challenge_id, notebook_id, attempt_number, status, started_at)
            VALUES ($1, $2, $3, $4, 'in_progress', NOW())
            RETURNING *
            "#
        )
        .bind(auth.user_id)
        .bind(challenge_id)
        .bind(notebook.id)
        .bind(next_attempt_number)
        .fetch_one(&state.pool)
        .await?;

        // Count first challenge engagement once
        if attempts_used == 0 {
            sqlx::query(
                "UPDATE user_stats SET challenges_taken = challenges_taken + 1, updated_at = NOW() WHERE user_id = $1"
            )
            .bind(auth.user_id)
            .execute(&state.pool)
            .await?;
        }

        (
            new_submission.id,
            new_submission.attempt_number,
            attempts_used + 1,
        )
    };

    let attempts_remaining = (allowed_submissions as i64 - attempts_used_after).max(0);

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
        attempt_number,
        attempts_used: attempts_used_after,
        attempts_remaining,
        token: jupyterhub_token,
    }))
}
