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

pub async fn admin_update_certificate(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<AdminUpdateCertificateRequest>,
) -> Result<Json<AdminItemResponse<AdminCertificateResponse>>, AppError> {
    let existing: Certificate = sqlx::query_as("SELECT * FROM certificates WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let level = req.level.unwrap_or(existing.level);
    let title = req.title.unwrap_or(existing.title);
    let cover_image = req.cover_image.or(existing.cover_image);
    let first_name = req.first_name.unwrap_or(existing.first_name);
    let second_name = req.second_name.unwrap_or(existing.second_name);
    let coursera_url = req.coursera_url.or(existing.coursera_url);
    let youtube_url = req.youtube_url.or(existing.youtube_url);
    let visible = req.visible.unwrap_or(existing.visible);

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
