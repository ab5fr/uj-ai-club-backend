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

pub async fn admin_get_certificate_by_id(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdminItemResponse<AdminCertificateResponse>>, AppError> {
    let certificate: Certificate = sqlx::query_as("SELECT * FROM certificates WHERE id = $1")
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
