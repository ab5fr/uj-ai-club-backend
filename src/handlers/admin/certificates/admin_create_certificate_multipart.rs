use axum::{Json, extract::State};

use crate::{AppState, auth::AdminUser, error::AppError, models::*};

use super::normalize_youtube_url::normalize_youtube_url;
use super::save_uploaded_file::save_uploaded_file;

pub async fn admin_create_certificate_multipart(
    _auth: AdminUser,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<AdminItemResponse<AdminCertificateResponse>>, AppError> {
    let mut level: Option<String> = None;
    let mut title: Option<String> = None;
    let mut course_title: Option<String> = None;
    let mut cover_image: Option<String> = None;
    let mut first_name: Option<String> = None;
    let mut second_name: Option<String> = None;
    let mut coursera_url: Option<String> = None;
    let mut youtube_url: Option<String> = None;
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
            "courseTitle" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                if !text.is_empty() {
                    course_title = Some(text);
                }
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
                if !text.is_empty() {
                    coursera_url = Some(text);
                }
            }
            "youtubeUrl" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                if !text.is_empty() {
                    youtube_url = Some(normalize_youtube_url(&text));
                }
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

    let level =
        level.ok_or_else(|| AppError::BadRequest("Missing required field: level".to_string()))?;
    let title =
        title.ok_or_else(|| AppError::BadRequest("Missing required field: title".to_string()))?;
    let course_title = course_title.ok_or_else(|| {
        AppError::BadRequest("Missing required field: courseTitle".to_string())
    })?;
    let first_name = first_name
        .ok_or_else(|| AppError::BadRequest("Missing required field: firstName".to_string()))?;
    let second_name = second_name
        .ok_or_else(|| AppError::BadRequest("Missing required field: secondName".to_string()))?;

    let certificate: Certificate = sqlx::query_as(
        r#"
        INSERT INTO certificates (level, title, course_title, cover_image, first_name, second_name, coursera_url, youtube_url, visible, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(&level)
    .bind(&title)
    .bind(&course_title)
    .bind(&cover_image)
    .bind(&first_name)
    .bind(&second_name)
    .bind(&coursera_url)
    .bind(&youtube_url)
    .bind(visible.unwrap_or(true))
    .fetch_one(&state.pool)
    .await?;

    let response = AdminCertificateResponse {
        id: certificate.id,
        level: certificate.level,
        title: certificate.title,
        course_title: certificate.course_title,
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
