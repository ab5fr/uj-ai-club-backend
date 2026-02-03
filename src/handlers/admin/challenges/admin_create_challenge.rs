use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

pub async fn admin_create_challenge(
    _auth: AdminUser,
    State(state): State<AppState>,
    Json(req): Json<AdminCreateChallengeRequest>,
) -> Result<Json<AdminItemResponse<AdminChallengeResponse>>, AppError> {
    let visible = req.visible.unwrap_or(true);
    let week = req.week.unwrap_or(1);
    let challenge_url = req.challenge_url.unwrap_or_default();

    let challenge: Challenge = sqlx::query_as(
        r#"
        INSERT INTO challenges (title, description, start_date, end_date, visible, week, challenge_url, is_current, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, false, NOW(), NOW())
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
    .fetch_one(&state.pool)
    .await?;

    let response = AdminChallengeResponse {
        id: challenge.id,
        title: challenge.title,
        description: challenge.description,
        start_date: challenge.start_date,
        end_date: challenge.end_date,
        visible: challenge.visible,
        created_at: challenge.created_at,
        updated_at: challenge.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
