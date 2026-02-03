use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

pub async fn get_current_challenge(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ChallengeResponse>, AppError> {
    let challenge: Challenge = sqlx::query_as(
        r#"
        SELECT * FROM challenges 
        WHERE visible = true 
        AND (start_date IS NULL OR start_date <= NOW())
        AND (end_date IS NULL OR end_date >= NOW())
        ORDER BY created_at DESC 
        LIMIT 1
        "#,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(ChallengeResponse {
        id: challenge.id,
        week: challenge.week,
        title: challenge.title,
        description: challenge.description,
        challenge_url: challenge.challenge_url,
    }))
}
