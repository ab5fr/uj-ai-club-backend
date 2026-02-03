use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

pub async fn update_user_profile(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UpdateProfileResponse>, AppError> {
    // Get current user data
    let current_user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let full_name = req.full_name.unwrap_or(current_user.full_name);
    let image = req.image.or(current_user.image);

    let updated_user: User = sqlx::query_as(
        r#"
        UPDATE users 
        SET full_name = $1, image = $2
        WHERE id = $3
        RETURNING *
        "#,
    )
    .bind(&full_name)
    .bind(&image)
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(UpdateProfileResponse {
        id: updated_user.id,
        full_name: updated_user.full_name,
        email: updated_user.email,
        image: updated_user.image,
        role: updated_user.role,
    }))
}
