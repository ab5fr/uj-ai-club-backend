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

pub async fn admin_patch_resource_visibility(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<AdminVisibilityRequest>,
) -> Result<Json<AdminItemResponse<AdminResourceResponse>>, AppError> {
    let resource: Resource = sqlx::query_as(
        "UPDATE resources SET visible = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
    )
    .bind(req.visible)
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
