use axum::{Json, extract::State};

use crate::{
    AppState,
    error::AppError,
    models::*,
    submissions::{apply_grade, ApplyGradeParams},
};

/// Webhook endpoint for nbgrader to report grades
pub async fn nbgrader_grade_webhook(
    State(state): State<AppState>,
    Json(payload): Json<NbgraderWebhookPayload>,
) -> Result<Json<NbgraderWebhookResponse>, AppError> {
    let expected_secret = std::env::var("NBGRADER_WEBHOOK_SECRET").map_err(|_| {
        AppError::InternalError(anyhow::anyhow!("NBGRADER_WEBHOOK_SECRET must be set"))
    })?;

    if expected_secret.is_empty() {
        return Err(AppError::InternalError(anyhow::anyhow!(
            "NBGRADER_WEBHOOK_SECRET must not be empty"
        )));
    }

    if !crate::security::constant_time_eq(&payload.webhook_secret, &expected_secret) {
        return Err(AppError::AuthError);
    }

    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE assignment_name = $1")
            .bind(&payload.assignment_name)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound)?;

    let user: User = sqlx::query_as("SELECT * FROM users WHERE jupyterhub_username = $1")
        .bind(&payload.student_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound)?;

    #[derive(sqlx::FromRow)]
    struct SubmissionRow {
        id: uuid::Uuid,
        manual_graded_at: Option<time::OffsetDateTime>,
    }

    let existing: Option<SubmissionRow> = sqlx::query_as(
        r#"
        SELECT id, manual_graded_at
        FROM challenge_submissions
        WHERE user_id = $1 AND challenge_id = $2
          AND status IN ('in_progress', 'grading_pending', 'graded')
        ORDER BY attempt_number DESC
        LIMIT 1
        "#,
    )
    .bind(user.id)
    .bind(notebook.challenge_id)
    .fetch_optional(&state.pool)
    .await?;

    let Some(existing) = existing else {
        return Err(AppError::NotFound);
    };

    // Never overwrite a manual grade with a late/async nbgrader callback.
    if existing.manual_graded_at.is_some() {
        return Ok(Json(NbgraderWebhookResponse {
            success: true,
            points_awarded: 0,
            message: "Skipped webhook: submission was manually graded".to_string(),
        }));
    }

    if let Some(ref nb_sub_id) = payload.submission_id {
        sqlx::query(
            "UPDATE challenge_submissions SET nbgrader_submission_id = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(nb_sub_id)
        .bind(existing.id)
        .execute(&state.pool)
        .await?;
    }

    if notebook.auto_grade_enabled {
        let graded = apply_grade(
            &state.pool,
            ApplyGradeParams {
                submission_id: existing.id,
                score: payload.score,
                max_score: payload.max_score,
                manual_graded_by: None,
            },
        )
        .await?;

        tracing::info!(
            "Auto-graded submission {} for {}: {:.1}% ({} pts)",
            existing.id,
            payload.student_id,
            graded.score.unwrap_or(0.0),
            graded.points_awarded
        );

        return Ok(Json(NbgraderWebhookResponse {
            success: true,
            points_awarded: graded.points_awarded,
            message: format!(
                "Auto-graded {} for {}",
                notebook.assignment_name, payload.student_id
            ),
        }));
    }

    // Manual grading path: store raw nbgrader score for admin review.
    let percentage = if payload.max_score > 0.0 {
        (payload.score / payload.max_score) * 100.0
    } else {
        payload.score
    };

    sqlx::query(
        r#"
        UPDATE challenge_submissions
        SET status = 'grading_pending',
            score = $1,
            max_score = 100.0,
            submitted_at = COALESCE(submitted_at, NOW()),
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(percentage)
    .bind(existing.id)
    .execute(&state.pool)
    .await?;

    Ok(Json(NbgraderWebhookResponse {
        success: true,
        points_awarded: 0,
        message: format!(
            "Submission received for {} and marked as grading_pending",
            notebook.assignment_name
        ),
    }))
}
