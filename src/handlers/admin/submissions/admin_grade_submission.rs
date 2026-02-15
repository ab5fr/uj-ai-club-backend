use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState, auth::AdminUser, error::AppError,
    handlers::webhooks::update_user_ranks::update_user_ranks, models::*,
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

    #[derive(sqlx::FromRow)]
    struct GradeTarget {
        user_id: uuid::Uuid,
        points_awarded: i32,
        points_credited: bool,
        max_points: i32,
        status: String,
    }

    let target: GradeTarget = sqlx::query_as(
        r#"
        SELECT
            cs.user_id,
            cs.points_awarded,
            cs.points_credited,
            cn.max_points,
            cs.status
        FROM challenge_submissions cs
        JOIN challenge_notebooks cn ON cn.id = cs.notebook_id
        WHERE cs.id = $1
        "#,
    )
    .bind(submission_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    if target.status != "grading_pending"
        && target.status != "graded"
        && target.status != "submitted"
        && target.status != "grading"
    {
        return Err(AppError::BadRequest(
            "Only submitted, grading, grading_pending, or graded submissions can be manually graded"
                .to_string(),
        ));
    }

    let points_awarded = ((req.score / 100.0) * target.max_points as f64).round() as i32;
    let delta_points = if target.points_credited {
        points_awarded - target.points_awarded
    } else {
        points_awarded
    };

    let updated_submission: ChallengeSubmission = sqlx::query_as(
        r#"
        UPDATE challenge_submissions
        SET status = 'graded',
            score = $1,
            max_score = 100.0,
            points_awarded = $2,
            points_credited = true,
            graded_at = NOW(),
            manual_graded_by = $3,
            manual_graded_at = NOW(),
            updated_at = NOW()
        WHERE id = $4
        RETURNING *
        "#,
    )
    .bind(req.score)
    .bind(points_awarded)
    .bind(auth.user_id)
    .bind(submission_id)
    .fetch_one(&state.pool)
    .await?;

    if delta_points != 0 {
        sqlx::query("UPDATE users SET points = points + $1 WHERE id = $2")
            .bind(delta_points)
            .bind(target.user_id)
            .execute(&state.pool)
            .await?;
    }

    update_user_ranks(&state.pool).await?;

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
