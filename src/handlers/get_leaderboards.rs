use axum::{Json, extract::State};

use crate::{AppState, error::AppError, models::*};

pub async fn get_leaderboards(
    State(state): State<AppState>,
) -> Result<Json<Vec<LeaderboardResponse>>, AppError> {
    // Get top 10 users by points
    let entries: Vec<LeaderboardEntry> =
        sqlx::query_as("SELECT full_name as name, points FROM users ORDER BY points DESC LIMIT 10")
            .fetch_all(&state.pool)
            .await?;

    // Return a single leaderboard with top 10 users
    let response = LeaderboardResponse {
        id: 1,
        title: "Top Users".to_string(),
        entries,
    };

    Ok(Json(vec![response]))
}
