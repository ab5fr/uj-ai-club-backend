use axum::{
    Json,
    extract::State,
};
use time::OffsetDateTime;

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
    submissions::effective_allowed_submissions,
};

/// Get all challenges with notebook information for the user
pub async fn get_challenges_with_notebooks(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<ChallengeWithNotebookResponse>>, AppError> {
    let challenges: Vec<Challenge> = sqlx::query_as(
        r#"
        SELECT * FROM challenges 
        WHERE visible = true 
        ORDER BY week DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let mut responses = Vec::new();

    for challenge in challenges {
        let allowed_submissions = effective_allowed_submissions(
            &state.pool,
            auth.user_id,
            challenge.id,
            challenge.allowed_submissions,
        )
        .await?;

        let notebook: Option<ChallengeNotebook> =
            sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
                .bind(challenge.id)
                .fetch_optional(&state.pool)
                .await?;

        let attempts_used: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM challenge_submissions WHERE user_id = $1 AND challenge_id = $2",
        )
        .bind(auth.user_id)
        .bind(challenge.id)
        .fetch_one(&state.pool)
        .await?;

        let latest: Option<ChallengeSubmission> = sqlx::query_as(
            r#"
            SELECT * FROM challenge_submissions
            WHERE user_id = $1 AND challenge_id = $2
            ORDER BY attempt_number DESC
            LIMIT 1
            "#,
        )
        .bind(auth.user_id)
        .bind(challenge.id)
        .fetch_optional(&state.pool)
        .await?;

        let now = OffsetDateTime::now_utc();
        let has_in_progress = latest
            .as_ref()
            .map(|s| s.status == "in_progress")
            .unwrap_or(false);

        let (submission_status, session_expires_at, can_submit, can_continue, can_start) =
            if let Some(s) = &latest {
                if s.status == "in_progress" {
                    let expired = s
                        .session_expires_at
                        .map(|e| now >= e)
                        .unwrap_or(false);
                    let revoked = s.session_revoked_at.is_some();
                    (
                        Some(s.status.clone()),
                        s.session_expires_at,
                        !revoked,
                        !expired && !revoked,
                        false,
                    )
                } else {
                    (
                        Some(s.status.clone()),
                        s.session_expires_at,
                        false,
                        false,
                        notebook.is_some()
                            && !has_in_progress
                            && attempts_used < allowed_submissions as i64,
                    )
                }
            } else {
                (
                    None,
                    None,
                    false,
                    false,
                    notebook.is_some() && attempts_used < allowed_submissions as i64,
                )
            };

        responses.push(ChallengeWithNotebookResponse {
            id: challenge.id,
            week: challenge.week,
            title: challenge.title,
            description: challenge.description,
            allowed_submissions,
            has_notebook: notebook.is_some(),
            max_points: notebook.as_ref().map(|n| n.max_points),
            time_limit_minutes: notebook.as_ref().map(|n| n.time_limit_minutes),
            start_date: challenge.start_date,
            end_date: challenge.end_date,
            submission_status,
            session_expires_at,
            can_start: can_start && notebook.is_some(),
            can_submit,
            can_continue,
            attempts_used,
            attempts_remaining: (allowed_submissions as i64 - attempts_used).max(0),
        });
    }

    Ok(Json(responses))
}
