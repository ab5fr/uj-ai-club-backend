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

pub async fn admin_get_resource_by_id(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdminItemResponse<AdminResourceResponse>>, AppError> {
    let resource: Resource = sqlx::query_as("SELECT * FROM resources WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let response = AdminResourceResponse {
        id: resource.id,
        title: resource.title,
        provider: resource.provider,
        cover_image: resource.cover_image,
        notion_url: resource.notion_url,
        instructor: Some(AdminInstructorResponse {
            name: resource.instructor_name,
            image: resource.instructor_image,
        }),
        quote: None,
        visible: resource.visible,
        created_at: resource.created_at,
        updated_at: resource.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
