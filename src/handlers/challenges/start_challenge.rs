use axum::{
    Json,
    extract::{Path, State},
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{self, AuthUser},
    error::AppError,
    jupyterhub,
    models::*,
    submissions::effective_allowed_submissions,
};

/// Start a challenge - creates submission record and returns JupyterHub URL
pub async fn start_challenge(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(challenge_id): Path<i32>,
) -> Result<Json<StartChallengeResponse>, AppError> {
    let challenge: Challenge =
        sqlx::query_as("SELECT * FROM challenges WHERE id = $1 AND visible = true")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    let allowed_submissions =
        effective_allowed_submissions(&state.pool, auth.user_id, challenge_id, challenge.allowed_submissions).await?;

    let now = OffsetDateTime::now_utc();
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

    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("This challenge does not have a notebook".to_string())
            })?;

    let _user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let jupyterhub_username = jupyterhub::student_jupyterhub_username(auth.user_id);

    sqlx::query(
        "UPDATE users SET jupyterhub_username = $1 WHERE id = $2 AND (jupyterhub_username IS NULL OR jupyterhub_username LIKE 'admin_%')",
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

    let (submission_id, attempt_number, attempts_used_after, started_at, force_fresh) =
        if let Some(existing) = existing_in_progress {
            let started = existing.started_at.unwrap_or(now);
            (
                existing.id,
                existing.attempt_number,
                attempts_used,
                started,
                false,
            )
        } else {
            if attempts_used >= allowed_submissions as i64 {
                return Err(AppError::BadRequest(format!(
                    "Submission limit reached for this challenge ({allowed_submissions} attempts)"
                )));
            }

            let next_attempt_number = attempts_used as i32 + 1;

            let new_submission: ChallengeSubmission = sqlx::query_as(
                r#"
                INSERT INTO challenge_submissions (user_id, challenge_id, notebook_id, attempt_number, status, started_at)
                VALUES ($1, $2, $3, $4, 'in_progress', NOW())
                RETURNING *
                "#,
            )
            .bind(auth.user_id)
            .bind(challenge_id)
            .bind(notebook.id)
            .bind(next_attempt_number)
            .fetch_one(&state.pool)
            .await?;

            if attempts_used == 0 {
                sqlx::query(
                    "UPDATE user_stats SET challenges_taken = challenges_taken + 1, updated_at = NOW() WHERE user_id = $1",
                )
                .bind(auth.user_id)
                .execute(&state.pool)
                .await?;
            }

            (
                new_submission.id,
                new_submission.attempt_number,
                attempts_used + 1,
                new_submission.started_at.unwrap_or(now),
                true,
            )
        };

    let session_expires_at = started_at
        + time::Duration::minutes(notebook.time_limit_minutes.max(1) as i64);
    if now >= session_expires_at {
        return Err(AppError::BadRequest(
            "Time limit expired for this attempt. Submit or start a new attempt.".to_string(),
        ));
    }

    let jti = Uuid::new_v4().to_string();
    let jupyterhub_token = auth::create_jupyterhub_session_token(
        auth.user_id,
        &jupyterhub_username,
        submission_id,
        &jti,
        session_expires_at,
    )?;

    sqlx::query(
        r#"
        UPDATE challenge_submissions
        SET session_jti = $1,
            session_expires_at = $2,
            session_revoked_at = NULL,
            updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(&jti)
    .bind(session_expires_at)
    .bind(submission_id)
    .execute(&state.pool)
    .await?;

    let attempts_remaining = (allowed_submissions as i64 - attempts_used_after).max(0);

    let grading_service_url = crate::grading::grading_service_url();
    let client = crate::grading::grading_client()?;
    let prepare_url = format!(
        "{}/prepare-notebook/{}/{}",
        grading_service_url, jupyterhub_username, notebook.assignment_name
    );

    match crate::grading::apply_grading_auth(
        client.post(&prepare_url).json(&serde_json::json!({
            "notebookPath": notebook.notebook_path,
            "notebookFilename": notebook.notebook_filename,
            "cpuLimit": notebook.cpu_limit,
            "memoryLimit": notebook.memory_limit,
            "networkDisabled": notebook.network_disabled,
            "forceFresh": force_fresh
        })),
    )?
    .send()
    .await
    {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!("Notebook prepared for user {jupyterhub_username}");
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                "Failed to prepare notebook for {jupyterhub_username}: {status} {body}"
            );
            return Err(AppError::InternalError(anyhow::anyhow!(
                "Failed to prepare notebook workspace before opening JupyterHub"
            )));
        }
        Err(e) => {
            tracing::error!("Could not reach grading service to prepare notebook: {e}");
            return Err(AppError::InternalError(anyhow::anyhow!(
                "Failed to prepare notebook workspace before opening JupyterHub"
            )));
        }
    }

    let jupyterhub_base_url = jupyterhub::jupyterhub_public_url();
    let next_path = format!(
        "/user/{jupyterhub_username}/notebooks/{}",
        notebook.notebook_filename
    );
    let encoded_next = urlencoding::encode(&next_path);
    let jupyterhub_url = format!(
        "{jupyterhub_base_url}/hub/login?token={jupyterhub_token}&next=/hub/spawn/{jupyterhub_username}?next={encoded_next}"
    );

    Ok(Json(StartChallengeResponse {
        success: true,
        jupyterhub_url,
        submission_id,
        attempt_number,
        attempts_used: attempts_used_after,
        attempts_remaining,
        token: jupyterhub_token,
        status: "in_progress".to_string(),
        session_expires_at,
    }))
}
