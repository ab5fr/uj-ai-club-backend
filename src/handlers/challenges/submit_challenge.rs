use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
    submissions::{
        FinalizeSubmissionParams, effective_allowed_submissions, finalize_in_progress_submission,
    },
};

/// Submit a challenge - marks submission as submitted and triggers grading
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

    let allowed_submissions = effective_allowed_submissions(
        &state.pool,
        auth.user_id,
        challenge_id,
        challenge.allowed_submissions,
    )
    .await?;

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

        if let Some(latest) = latest_submission
            && latest.status == "grading_pending"
        {
            return Ok(Json(SubmitChallengeResponse {
                success: true,
                message: "Your submission is pending grading.".to_string(),
                status: "grading_pending".to_string(),
                attempt_number: latest.attempt_number,
                attempts_used,
                attempts_remaining,
            }));
        }

        return Err(AppError::BadRequest(
            "No in-progress attempt found. Start the challenge before submitting.".to_string(),
        ));
    };

    if submission.session_revoked_at.is_some() {
        return Err(AppError::BadRequest(
            "This attempt has already been submitted.".to_string(),
        ));
    }

    let message = finalize_in_progress_submission(
        &state.pool,
        FinalizeSubmissionParams {
            submission_id: submission.id,
            user_id: auth.user_id,
            notebook_filename: &notebook.notebook_filename,
            notebook_path: &notebook.notebook_path,
            assignment_name: &notebook.assignment_name,
            auto_grade_enabled: notebook.auto_grade_enabled,
        },
    )
    .await?
    .unwrap_or_else(|| "Your submission is pending grading.".to_string());

    Ok(Json(SubmitChallengeResponse {
        success: true,
        message,
        status: "grading_pending".to_string(),
        attempt_number: submission.attempt_number,
        attempts_used,
        attempts_remaining,
    }))
}
