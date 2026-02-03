use axum::{
    Json,
    extract::{State},
};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

use super::save_uploaded_file::save_uploaded_file;

// Admin resource endpoints with multipart form data

pub async fn admin_create_resource_multipart(
    _auth: AdminUser,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<AdminItemResponse<AdminResourceResponse>>, AppError> {
    tracing::info!("Starting multipart resource creation");

    let mut title: Option<String> = None;
    let mut provider: Option<String> = None;
    let mut cover_image: Option<String> = None;
    let mut notion_url: Option<String> = None;
    let mut instructor_name: Option<String> = None;
    let mut instructor_image: Option<String> = None;
    let mut visible: Option<bool> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Error reading multipart field: {}", e);
        AppError::InternalError(e.into())
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        tracing::info!("Processing field: {}", field_name);

        match field_name.as_str() {
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?,
                );
            }
            "provider" => {
                provider = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?,
                );
            }
            "notionUrl" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                if !text.is_empty() {
                    notion_url = Some(text);
                }
            }
            "instructorName" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                if !text.is_empty() {
                    instructor_name = Some(text);
                }
            }
            "quoteText" | "quoteAuthor" => {
                // Ignore quote fields - quotes are now in a separate table
                let _ = field.text().await;
            }
            "visible" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                visible = Some(text == "true" || text == "1");
            }
            "coverImage" => {
                if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                    let data = field
                        .bytes()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?;
                    let url =
                        save_uploaded_file("coverImage", &file_name, &data, "resources/covers")
                            .await?;
                    cover_image = Some(url);
                }
            }
            "instructorImage" => {
                if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                    let data = field
                        .bytes()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?;
                    let url = save_uploaded_file(
                        "instructorImage",
                        &file_name,
                        &data,
                        "resources/instructors",
                    )
                    .await?;
                    instructor_image = Some(url);
                }
            }
            _ => {}
        }
    }

    let title =
        title.ok_or_else(|| AppError::BadRequest("Missing required field: title".to_string()))?;
    let provider = provider
        .ok_or_else(|| AppError::BadRequest("Missing required field: provider".to_string()))?;
    let instructor_name = instructor_name.unwrap_or_default();
    let visible = visible.unwrap_or(true);

    let resource: Resource = sqlx::query_as(
        r#"
        INSERT INTO resources (title, provider, cover_image, notion_url, instructor_name, instructor_image, visible, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
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
