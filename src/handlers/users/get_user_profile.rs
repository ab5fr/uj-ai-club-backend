use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

pub async fn get_user_profile(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UserProfileResponse>, AppError> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let stats: UserStats = sqlx::query_as("SELECT * FROM user_stats WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(UserProfileResponse {
        rank: user.rank,
        name: user.full_name,
        points: user.points,
        image: user.image,
        stats: UserStatsResponse {
            best_subject: stats.best_subject,
            improveable: stats.improveable,
            quickest_hunter: stats.quickest_hunter,
            challenges_taken: stats.challenges_taken,
        },
    }))
}
