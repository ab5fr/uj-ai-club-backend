use axum::{
    Json,
    extract::{Path, State},
};

use crate::{AppState, error::AppError, models::*};

pub async fn get_certificate_by_id(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<CertificateDetailResponse>, AppError> {
    let certificate: Certificate =
        sqlx::query_as("SELECT * FROM certificates WHERE id = $1 AND visible = true")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    let quote: Option<Quote> =
        sqlx::query_as("SELECT * FROM quotes WHERE visible = true ORDER BY RANDOM() LIMIT 1")
            .fetch_optional(&state.pool)
            .await?;

    let quote_response = quote.map(|q| QuoteResponse {
        text: q.text,
        author: q.author,
    });

    Ok(Json(CertificateDetailResponse {
        id: certificate.id,
        level: certificate.level,
        title: certificate.title,
        cover_image: certificate.cover_image,
        first_name: certificate.first_name,
        second_name: certificate.second_name,
        coursera_url: certificate.coursera_url,
        youtube_url: certificate.youtube_url,
        quote: quote_response,
    }))
}
