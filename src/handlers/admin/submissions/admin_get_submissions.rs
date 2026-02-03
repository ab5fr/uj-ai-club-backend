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
            cs.challenge_id, c.title as challenge_title,
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
        .map(|s| AdminSubmissionResponse {
            id: s.id,
            user_id: s.user_id,
            user_name: s.user_name,
            user_email: s.user_email,
            challenge_id: s.challenge_id,
            challenge_title: s.challenge_title,
            status: s.status,
            score: s.score,
            max_score: s.max_score,
            points_awarded: s.points_awarded,
            points_credited: s.points_credited,
            started_at: s.started_at,
            submitted_at: s.submitted_at,
            graded_at: s.graded_at,
        })
        .collect();

    Ok(Json(AdminItemsResponse { items: responses }))
}
