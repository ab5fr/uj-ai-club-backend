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
        WITH ranked_attempts AS (
            SELECT
                cs.*,
                ROW_NUMBER() OVER (
                    PARTITION BY cs.user_id
                    ORDER BY cs.points_awarded DESC, cs.graded_at DESC NULLS LAST, cs.created_at DESC
                ) AS rn
            FROM challenge_submissions cs
            WHERE cs.challenge_id = $1 AND cs.status = 'graded' AND cs.points_awarded > 0
        )
        SELECT 
            ra.challenge_id,
            u.id as user_id,
            u.full_name,
            u.image,
            ra.points_awarded,
            ra.score,
            ra.max_score,
            ra.status,
            ra.graded_at,
            RANK() OVER (ORDER BY ra.points_awarded DESC) as challenge_rank
        FROM ranked_attempts ra
        JOIN users u ON ra.user_id = u.id
        WHERE ra.rn = 1
        ORDER BY ra.points_awarded DESC
        LIMIT 50
        "#,
    )
    .bind(challenge_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(entries))
}
