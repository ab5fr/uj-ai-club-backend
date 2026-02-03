use axum::{Json, extract::State};
use bcrypt::{DEFAULT_COST, hash, verify};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

pub async fn update_user_password(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdatePasswordRequest>,
) -> Result<Json<UpdatePasswordResponse>, AppError> {
    // Get current user
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    // Check if user has a password (not a Google OAuth-only user)
    let current_password_hash = user.password_hash.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "This account uses Google Sign-In and doesn't have a password.".to_string(),
        )
    })?;

    // Verify current password
    if !verify(req.current_password.as_bytes(), current_password_hash)
        .map_err(|e| AppError::InternalError(e.into()))?
    {
        return Err(AppError::AuthError);
    }

    // Hash new password
    let new_password_hash = hash(req.new_password.as_bytes(), DEFAULT_COST)
        .map_err(|e| AppError::InternalError(e.into()))?;

    // Update password
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(new_password_hash)
        .bind(auth.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(UpdatePasswordResponse { success: true }))
}
