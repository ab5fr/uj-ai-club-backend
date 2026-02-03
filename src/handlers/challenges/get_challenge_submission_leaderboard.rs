use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

/// Get challenge submission leaderboard
pub async fn get_challenge_submission_leaderboard(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(challenge_id): Path<i32>,
) -> Result<Json<Vec<ChallengeSubmissionLeaderboardEntry>>, AppError> {
    let entries: Vec<ChallengeSubmissionLeaderboardEntry> = sqlx::query_as(
        r#"
        SELECT 
            cs.challenge_id,
            u.id as user_id,
            u.full_name,
            u.image,
            cs.points_awarded,
            cs.score,
            cs.max_score,
            cs.status,
            cs.graded_at,
            RANK() OVER (ORDER BY cs.points_awarded DESC) as challenge_rank
        FROM challenge_submissions cs
        JOIN users u ON cs.user_id = u.id
        WHERE cs.challenge_id = $1 AND cs.status = 'graded' AND cs.points_awarded > 0
        ORDER BY cs.points_awarded DESC
        LIMIT 50
        "#,
    )
    .bind(challenge_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(entries))
}
