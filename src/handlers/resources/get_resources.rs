use axum::{Json, extract::State};

use crate::{AppState, error::AppError, models::*};

pub async fn get_resources(
    State(state): State<AppState>,
) -> Result<Json<Vec<ResourceListResponse>>, AppError> {
    let resources: Vec<Resource> =
        sqlx::query_as("SELECT * FROM resources WHERE visible = true ORDER BY id")
            .fetch_all(&state.pool)
            .await?;

    let responses: Vec<ResourceListResponse> = resources
        .into_iter()
        .map(|r| ResourceListResponse {
            id: r.id,
            title: r.title,
            provider: r.provider,
            cover_image: r.cover_image,
            instructor: InstructorResponse {
                name: r.instructor_name,
                image: r.instructor_image,
            },
        })
        .collect();

    Ok(Json(responses))
}
