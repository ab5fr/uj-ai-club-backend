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

pub async fn admin_update_resource(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<AdminUpdateResourceRequest>,
) -> Result<Json<AdminItemResponse<AdminResourceResponse>>, AppError> {
    let existing: Resource = sqlx::query_as("SELECT * FROM resources WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let title = req.title.unwrap_or(existing.title);
    let provider = req.provider.unwrap_or(existing.provider);
    let cover_image = req.cover_image.or(existing.cover_image);
    let notion_url = req.notion_url.or(existing.notion_url);
    let instructor_name = req
        .instructor
        .as_ref()
        .map(|i| i.name.clone())
        .unwrap_or(existing.instructor_name);
    let instructor_image = req
        .instructor
        .as_ref()
        .and_then(|i| i.image.clone())
        .or(existing.instructor_image);
    let visible = req.visible.unwrap_or(existing.visible);

    let resource: Resource = sqlx::query_as(
        r#"
        UPDATE resources 
        SET title = $1, provider = $2, cover_image = $3, notion_url = $4, instructor_name = $5, instructor_image = $6, visible = $7, updated_at = NOW()
        WHERE id = $8
        RETURNING *
        "#,
    )
    .bind(&title)
    .bind(&provider)
    .bind(&cover_image)
    .bind(&notion_url)
    .bind(&instructor_name)
    .bind(&instructor_image)
    .bind(visible)
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

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
