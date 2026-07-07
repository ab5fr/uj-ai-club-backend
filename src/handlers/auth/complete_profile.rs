use axum::{Json, extract::State};

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
    let full_name = req.full_name.trim();
    if full_name.is_empty() {
        return Err(AppError::BadRequest("Full name is required".to_string()));
    }

    sqlx::query(
        "UPDATE users SET full_name = $1, university = $2, major = $3, university_major_set = TRUE WHERE id = $4",
    )
    .bind(full_name)
    .bind(&req.university)
    .bind(&req.major)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(CompleteProfileResponse { success: true }))
}
