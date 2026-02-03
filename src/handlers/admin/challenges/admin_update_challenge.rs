use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

pub async fn admin_update_challenge(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<AdminUpdateChallengeRequest>,
) -> Result<Json<AdminItemResponse<AdminChallengeResponse>>, AppError> {
    let existing: Challenge = sqlx::query_as("SELECT * FROM challenges WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let title = req.title.unwrap_or(existing.title);
    let description = req.description.unwrap_or(existing.description);
    let week = req.week.unwrap_or(existing.week);
    let challenge_url = req.challenge_url.unwrap_or(existing.challenge_url);
    let start_date = req.start_date.or(existing.start_date);
    let end_date = req.end_date.or(existing.end_date);
    let visible = req.visible.unwrap_or(existing.visible);

    let challenge: Challenge = sqlx::query_as(
        r#"
        UPDATE challenges 
        SET title = $1, description = $2, week = $3, challenge_url = $4, start_date = $5, end_date = $6, visible = $7, updated_at = NOW()
        WHERE id = $8
        RETURNING *
        "#,
    )
    .bind(&title)
    .bind(&description)
    .bind(week)
    .bind(&challenge_url)
    .bind(start_date)
    .bind(end_date)
    .bind(visible)
    .bind(id)
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
