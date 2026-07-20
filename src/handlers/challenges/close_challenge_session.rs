use axum::{
    Json,
    extract::{Path, State},
};
use time::OffsetDateTime;

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    grading,
    jupyterhub,
    models::*,
};

/// Save the student's notebook and stop Jupyter when access should end
/// (timer expiry, tab close follow-up, etc.). Does not submit for grading.
pub async fn close_challenge_session(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(challenge_id): Path<i32>,
) -> Result<Json<CloseChallengeSessionResponse>, AppError> {
    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("This challenge does not have a notebook".to_string())
            })?;

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

    let Some(submission) = submission else {
        return Ok(Json(CloseChallengeSessionResponse {
            success: true,
            message: "No active notebook session to close.".to_string(),
            server_stopped: false,
        }));
    };

    if submission.session_revoked_at.is_some() {
        return Ok(Json(CloseChallengeSessionResponse {
            success: true,
            message: "Notebook session is already closed.".to_string(),
            server_stopped: false,
        }));
    }

    let now = OffsetDateTime::now_utc();
    let expired = submission
        .session_expires_at
        .map(|expires| now >= expires)
        .unwrap_or(false);

    if !expired {
        return Err(AppError::BadRequest(
            "Your notebook session is still active.".to_string(),
        ));
    }

    let jupyterhub_username = jupyterhub::student_jupyterhub_username(auth.user_id);
    let server_running = jupyterhub::user_server_running(&jupyterhub_username).await?;

    if server_running {
        grading::close_jupyter_session(&jupyterhub_username, &notebook.notebook_filename).await?;
    }

    // Invalidate SSO token so Hub cannot re-login; spawn is also blocked once
    // session_expires_at has passed. Keep session_revoked_at NULL so submit still works.
    sqlx::query(
        r#"
        UPDATE challenge_submissions
        SET session_jti = NULL, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(submission.id)
    .execute(&state.pool)
    .await?;

    Ok(Json(CloseChallengeSessionResponse {
        success: true,
        message: if server_running {
            "Time is up. Your notebook was saved and your Jupyter session was closed. You can still submit your work.".to_string()
        } else {
            "Time is up. Your notebook session is already closed. You can still submit your work.".to_string()
        },
        server_stopped: server_running,
    }))
}
