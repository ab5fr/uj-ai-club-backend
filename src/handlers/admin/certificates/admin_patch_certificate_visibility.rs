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

pub async fn admin_patch_certificate_visibility(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<AdminVisibilityRequest>,
) -> Result<Json<AdminItemResponse<AdminCertificateResponse>>, AppError> {
    let certificate: Certificate = sqlx::query_as(
        "UPDATE certificates SET visible = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
    )
    .bind(req.visible)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

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
