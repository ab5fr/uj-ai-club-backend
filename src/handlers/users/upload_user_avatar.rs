use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    handlers::admin::upload::{sanitize_filename, validate_image_upload},
    models::*,
};

pub async fn upload_user_avatar(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadAvatarResponse>, AppError> {
    use tokio::io::AsyncWriteExt;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "avatar" {
            let file_name = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}.jpg", Uuid::new_v4()));

            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::InternalError(e.into()))?;

            validate_image_upload(&file_name, &data)?;

            tokio::fs::create_dir_all("uploads/avatars")
                .await
                .map_err(|e| AppError::InternalError(e.into()))?;

            let safe_name = sanitize_filename(&file_name);
            let unique_filename = format!("{}_{}", Uuid::new_v4(), safe_name);
            let file_path = format!("uploads/avatars/{unique_filename}");

            let mut file = tokio::fs::File::create(&file_path)
                .await
                .map_err(|e| AppError::InternalError(e.into()))?;

            file.write_all(&data)
                .await
                .map_err(|e| AppError::InternalError(e.into()))?;

            let image_url = format!("/uploads/avatars/{unique_filename}");

            sqlx::query("UPDATE users SET image = $1 WHERE id = $2")
                .bind(&image_url)
                .bind(auth.user_id)
                .execute(&state.pool)
                .await?;

            return Ok(Json(UploadAvatarResponse { image_url }));
        }
    }

    Err(AppError::BadRequest("No avatar file provided".to_string()))
}
