use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

fn sanitize_profile_image(image: Option<String>) -> Result<Option<String>, AppError> {
    let Some(image) = image else {
        return Ok(None);
    };

    let trimmed = image.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    // Only allow locally hosted avatar paths to prevent arbitrary remote/image URL injection.
    if trimmed.starts_with("/uploads/avatars/")
        && !trimmed.contains("..")
        && !trimmed.contains('\\')
        && trimmed.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-')
        })
    {
        return Ok(Some(trimmed.to_string()));
    }

    Err(AppError::BadRequest(
        "Image must be an uploaded avatar path under /uploads/avatars/".to_string(),
    ))
}

pub async fn update_user_profile(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UpdateProfileResponse>, AppError> {
    let current_user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let full_name = req.full_name.unwrap_or(current_user.full_name);
    let image = if req.image.is_some() {
        sanitize_profile_image(req.image)?
    } else {
        current_user.image
    };

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
