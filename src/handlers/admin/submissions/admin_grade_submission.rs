use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState, auth::AdminUser, error::AppError,
    submissions::{apply_grade, ApplyGradeParams},
    models::*,
};

pub async fn admin_grade_submission(
    auth: AdminUser,
    State(state): State<AppState>,
    Path(submission_id): Path<uuid::Uuid>,
    Json(req): Json<AdminGradeSubmissionRequest>,
) -> Result<Json<AdminItemResponse<AdminSubmissionResponse>>, AppError> {
    if !(0.0..=100.0).contains(&req.score) {
        return Err(AppError::BadRequest(
            "score must be between 0 and 100".to_string(),
        ));
    }

    let status: Option<(String,)> =
        sqlx::query_as("SELECT status FROM challenge_submissions WHERE id = $1")
            .bind(submission_id)
            .fetch_optional(&state.pool)
            .await?;

    let status = status.ok_or(AppError::NotFound)?.0;

    if status != "grading_pending"
        && status != "graded"
        && status != "submitted"
        && status != "grading"
    {
        return Err(AppError::BadRequest(
            "Only submitted, grading, grading_pending, or graded submissions can be manually graded"
                .to_string(),
        ));
    }

    let updated_submission = apply_grade(
        &state.pool,
        ApplyGradeParams {
            submission_id,
            score: req.score,
            max_score: 100.0,
            manual_graded_by: Some(auth.user_id),
        },
    )
    .await?;

    #[derive(sqlx::FromRow)]
    struct AdminSubmissionRow {
        id: uuid::Uuid,
        user_id: uuid::Uuid,
        user_name: String,
        user_email: String,
        challenge_id: i32,
        challenge_title: String,
        allowed_submissions: i32,
        attempt_number: i32,
        attempts_used: i64,
        status: String,
        score: Option<f64>,
        max_score: Option<f64>,
        points_awarded: i32,
        points_credited: bool,
        started_at: Option<time::OffsetDateTime>,
        submitted_at: Option<time::OffsetDateTime>,
        graded_at: Option<time::OffsetDateTime>,
    }

    let response_row: AdminSubmissionRow = sqlx::query_as(
        r#"
        SELECT
            cs.id,
            cs.user_id,
            u.full_name AS user_name,
            u.email AS user_email,
            cs.challenge_id,
            c.title AS challenge_title,
            c.allowed_submissions,
            cs.attempt_number,
            COUNT(*) OVER (PARTITION BY cs.user_id, cs.challenge_id) AS attempts_used,
            cs.status,
            cs.score,
            cs.max_score,
            cs.points_awarded,
            cs.points_credited,
            cs.started_at,
            cs.submitted_at,
            cs.graded_at
        FROM challenge_submissions cs
        JOIN users u ON u.id = cs.user_id
        JOIN challenges c ON c.id = cs.challenge_id
        WHERE cs.id = $1
        "#,
    )
    .bind(updated_submission.id)
    .fetch_one(&state.pool)
    .await?;

    let allowed_submissions = response_row.allowed_submissions.max(1);

    Ok(Json(AdminItemResponse {
        item: AdminSubmissionResponse {
            id: response_row.id,
            user_id: response_row.user_id,
            user_name: response_row.user_name,
            user_email: response_row.user_email,
            challenge_id: response_row.challenge_id,
            challenge_title: response_row.challenge_title,
            allowed_submissions,
            attempt_number: response_row.attempt_number,
            attempts_used: response_row.attempts_used,
            attempts_remaining: (allowed_submissions as i64 - response_row.attempts_used).max(0),
            status: response_row.status,
            score: response_row.score,
            max_score: response_row.max_score,
            points_awarded: response_row.points_awarded,
            points_credited: response_row.points_credited,
            started_at: response_row.started_at,
            submitted_at: response_row.submitted_at,
            graded_at: response_row.graded_at,
        },
    }))
}
