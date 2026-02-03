use axum::{
    Json,
    extract::{Path, State},
};

use crate::{AppState, error::AppError, models::*};

pub async fn get_resource_by_id(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ResourceDetailResponse>, AppError> {
    let resource: Resource =
        sqlx::query_as("SELECT * FROM resources WHERE id = $1 AND visible = true")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    // Fetch a random quote from the quotes table
    let quote: Option<Quote> =
        sqlx::query_as("SELECT * FROM quotes WHERE visible = true ORDER BY RANDOM() LIMIT 1")
            .fetch_optional(&state.pool)
            .await?;

    let quote_response = quote.map(|q| QuoteResponse {
        text: q.text,
        author: q.author,
    });

    Ok(Json(ResourceDetailResponse {
        id: resource.id,
        title: resource.title,
        provider: resource.provider,
        notion_url: resource.notion_url,
        instructor: InstructorResponse {
            name: resource.instructor_name,
            image: resource.instructor_image,
        },
        quote: quote_response,
    }))
}
