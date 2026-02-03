use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

pub async fn admin_create_resource(
    _auth: AdminUser,
    State(state): State<AppState>,
    Json(req): Json<AdminCreateResourceRequest>,
) -> Result<Json<AdminItemResponse<AdminResourceResponse>>, AppError> {
    let visible = req.visible.unwrap_or(true);
    let instructor_name = req
        .instructor
        .as_ref()
        .map(|i| i.name.clone())
        .unwrap_or_default();
    let instructor_image = req.instructor.as_ref().and_then(|i| i.image.clone());

    let resource: Resource = sqlx::query_as(
        r#"
        INSERT INTO resources (title, provider, cover_image, notion_url, instructor_name, instructor_image, visible, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(&req.title)
    .bind(&req.provider)
    .bind(&req.cover_image)
    .bind(&req.notion_url)
    .bind(&instructor_name)
    .bind(&instructor_image)
    .bind(visible)
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
