use axum::{Json, extract::State};

use crate::{AppState, error::AppError, models::*};

pub async fn get_certificates(
    State(state): State<AppState>,
) -> Result<Json<Vec<CertificateListResponse>>, AppError> {
    let certificates: Vec<Certificate> =
        sqlx::query_as("SELECT * FROM certificates WHERE visible = true ORDER BY id")
            .fetch_all(&state.pool)
            .await?;

    let responses: Vec<CertificateListResponse> = certificates
        .into_iter()
        .map(|c| CertificateListResponse {
            id: c.id,
            level: c.level,
            title: c.title,
            cover_image: c.cover_image,
            first_name: c.first_name,
            second_name: c.second_name,
        })
        .collect();

    Ok(Json(responses))
}
