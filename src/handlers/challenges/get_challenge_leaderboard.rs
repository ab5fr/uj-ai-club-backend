use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

pub async fn get_challenge_leaderboard(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<ChallengeLeaderboardEntry>>, AppError> {
    
    let entries: Vec<ChallengeLeaderboardEntry> = sqlx::query_as(
        r#"
        SELECT id, full_name as name, points, image
        FROM users
        ORDER BY points DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(entries))
}
