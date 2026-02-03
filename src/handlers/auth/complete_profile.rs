use axum::{Json, extract::State};
use bcrypt::{DEFAULT_COST, hash};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

pub async fn complete_profile(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CompleteProfileRequest>,
) -> Result<Json<CompleteProfileResponse>, AppError> {
    // Hash the password
    let password_hash = hash(req.password.as_bytes(), DEFAULT_COST)
        .map_err(|e| AppError::InternalError(e.into()))?;

    // Update user's university, major, and password
    sqlx::query(
        "UPDATE users SET university = $1, major = $2, university_major_set = TRUE, password_hash = $3 WHERE id = $4",
    )
    .bind(&req.university)
    .bind(&req.major)
    .bind(&password_hash)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(CompleteProfileResponse { success: true }))
}
