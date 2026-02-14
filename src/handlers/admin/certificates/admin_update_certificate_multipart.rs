use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

use super::normalize_youtube_url::normalize_youtube_url;
use super::save_uploaded_file::save_uploaded_file;

pub async fn admin_update_certificate_multipart(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<AdminItemResponse<AdminCertificateResponse>>, AppError> {
    let existing: Certificate = sqlx::query_as("SELECT * FROM certificates WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut level: Option<String> = None;
    let mut title: Option<String> = None;
    let mut cover_image: Option<String> = None;
    let mut first_name: Option<String> = None;
    let mut second_name: Option<String> = None;
    let mut coursera_url: Option<Option<String>> = None;
    let mut youtube_url: Option<Option<String>> = None;
    let mut visible: Option<bool> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "level" => {
                level = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?,
                );
            }
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?,
                );
            }
            "firstName" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                if !text.is_empty() {
                    first_name = Some(text);
                }
            }
            "secondName" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                if !text.is_empty() {
                    second_name = Some(text);
                }
            }
            "courseraUrl" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                coursera_url = Some(if text.is_empty() { None } else { Some(text) });
            }
            "youtubeUrl" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                youtube_url = Some(if text.is_empty() {
                    None
                } else {
                    Some(normalize_youtube_url(&text))
                });
            }
            "visible" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                visible = Some(text == "true" || text == "1");
            }
            "coverImage" => {
                if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                    let data = field
                        .bytes()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?;
                    let url =
                        save_uploaded_file("coverImage", &file_name, &data, "certificates/covers")
                            .await?;
                    cover_image = Some(url);
                }
            }
            _ => {}
        }
    }

    let level = level.unwrap_or(existing.level);
    let title = title.unwrap_or(existing.title);
    let cover_image = cover_image.or(existing.cover_image);
    let first_name = first_name.unwrap_or(existing.first_name);
    let second_name = second_name.unwrap_or(existing.second_name);
    let coursera_url = coursera_url.unwrap_or(existing.coursera_url);
    let youtube_url = youtube_url.unwrap_or(existing.youtube_url);
    let visible = visible.unwrap_or(existing.visible);

    let certificate: Certificate = sqlx::query_as(
        r#"
        UPDATE certificates
        SET level = $1, title = $2, cover_image = $3, first_name = $4, second_name = $5, coursera_url = $6, youtube_url = $7, visible = $8, updated_at = NOW()
        WHERE id = $9
        RETURNING *
        "#,
    )
    .bind(&level)
    .bind(&title)
    .bind(&cover_image)
    .bind(&first_name)
    .bind(&second_name)
    .bind(&coursera_url)
    .bind(&youtube_url)
    .bind(visible)
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    let response = AdminCertificateResponse {
        id: certificate.id,
        level: certificate.level,
        title: certificate.title,
        cover_image: certificate.cover_image,
        first_name: certificate.first_name,
        second_name: certificate.second_name,
        coursera_url: certificate.coursera_url,
        youtube_url: certificate.youtube_url,
        visible: certificate.visible,
        created_at: certificate.created_at,
        updated_at: certificate.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
