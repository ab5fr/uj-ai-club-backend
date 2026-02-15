use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

/// Get all submissions (admin)
pub async fn admin_get_submissions(
    _auth: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<AdminItemsResponse<AdminSubmissionResponse>>, AppError> {
    #[derive(sqlx::FromRow)]
    struct SubmissionRow {
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

    let submissions: Vec<SubmissionRow> = sqlx::query_as(
        r#"
        SELECT 
            cs.id, cs.user_id, u.full_name as user_name, u.email as user_email,
            cs.challenge_id, c.title as challenge_title, c.allowed_submissions,
            cs.attempt_number,
            COUNT(*) OVER (PARTITION BY cs.user_id, cs.challenge_id) AS attempts_used,
            cs.status, cs.score, cs.max_score, cs.points_awarded, cs.points_credited,
            cs.started_at, cs.submitted_at, cs.graded_at
        FROM challenge_submissions cs
        JOIN users u ON cs.user_id = u.id
        JOIN challenges c ON cs.challenge_id = c.id
        ORDER BY cs.created_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let responses: Vec<AdminSubmissionResponse> = submissions
        .into_iter()
        .map(|s| {
            let allowed_submissions = s.allowed_submissions.max(1);

            AdminSubmissionResponse {
                id: s.id,
                user_id: s.user_id,
                user_name: s.user_name,
                user_email: s.user_email,
                challenge_id: s.challenge_id,
                challenge_title: s.challenge_title,
                allowed_submissions,
                attempt_number: s.attempt_number,
                attempts_used: s.attempts_used,
                attempts_remaining: (allowed_submissions as i64 - s.attempts_used).max(0),
                status: s.status,
                score: s.score,
                max_score: s.max_score,
                points_awarded: s.points_awarded,
                points_credited: s.points_credited,
                started_at: s.started_at,
                submitted_at: s.submitted_at,
                graded_at: s.graded_at,
            }
        })
        .collect();

    Ok(Json(AdminItemsResponse { items: responses }))
}
