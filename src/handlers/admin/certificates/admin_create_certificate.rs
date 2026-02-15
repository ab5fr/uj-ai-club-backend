use axum::{Json, extract::State};

use crate::{AppState, auth::AdminUser, error::AppError, models::*};

use super::normalize_youtube_url::normalize_youtube_url;

pub async fn admin_create_certificate(
    _auth: AdminUser,
    State(state): State<AppState>,
    Json(req): Json<AdminCreateCertificateRequest>,
) -> Result<Json<AdminItemResponse<AdminCertificateResponse>>, AppError> {
    let visible = req.visible.unwrap_or(true);
    let youtube_url = req.youtube_url.map(|url| normalize_youtube_url(&url));

    let certificate: Certificate = sqlx::query_as(
        r#"
        INSERT INTO certificates (level, title, course_title, cover_image, first_name, second_name, coursera_url, youtube_url, visible, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(&req.level)
    .bind(&req.title)
    .bind(&req.course_title)
    .bind(&req.cover_image)
    .bind(&req.first_name)
    .bind(&req.second_name)
    .bind(&req.coursera_url)
    .bind(&youtube_url)
    .bind(visible)
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
