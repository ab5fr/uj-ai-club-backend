use axum::{Json, extract::State};

use crate::{AppState, auth::AdminUser, error::AppError, models::*};

pub async fn admin_create_challenge(
    _auth: AdminUser,
    State(state): State<AppState>,
    Json(req): Json<AdminCreateChallengeRequest>,
) -> Result<Json<AdminItemResponse<AdminChallengeResponse>>, AppError> {
    let visible = req.visible.unwrap_or(true);
    let week = req.week.unwrap_or(1);
    let challenge_url = req.challenge_url.unwrap_or_default();
    let allowed_submissions = req.allowed_submissions.unwrap_or(3);

    if allowed_submissions < 1 {
        return Err(AppError::BadRequest(
            "allowedSubmissions must be at least 1".to_string(),
        ));
    }

    let challenge: Challenge = sqlx::query_as(
        r#"
        INSERT INTO challenges (title, description, start_date, end_date, visible, week, challenge_url, allowed_submissions, is_current, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.start_date)
    .bind(req.end_date)
    .bind(visible)
    .bind(week)
    .bind(&challenge_url)
    .bind(allowed_submissions)
    .fetch_one(&state.pool)
    .await?;

    let response = AdminChallengeResponse {
        id: challenge.id,
        title: challenge.title,
        description: challenge.description,
        allowed_submissions: challenge.allowed_submissions,
        start_date: challenge.start_date,
        end_date: challenge.end_date,
        visible: challenge.visible,
        created_at: challenge.created_at,
        updated_at: challenge.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
