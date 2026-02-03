use axum::{Json, extract::State};
use bcrypt::verify;

use crate::{AppState, auth::create_token, error::AppError, models::*};

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE email = $1")
        .bind(req.email)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::AuthError)?;

    let password_hash = user.password_hash.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "This account uses Google Sign-In. Please use the 'Sign in with Google' button."
                .to_string(),
        )
    })?;

    if !verify(req.password.as_bytes(), password_hash)
        .map_err(|e| AppError::InternalError(e.into()))?
    {
        return Err(AppError::AuthError);
    }

    let token = create_token(user.id)?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            full_name: user.full_name,
            email: user.email,
            image: user.image,
            role: user.role,
        },
    }))
}
