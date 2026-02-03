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

use super::save_uploaded_file::save_uploaded_file;

pub async fn admin_update_resource_multipart(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<AdminItemResponse<AdminResourceResponse>>, AppError> {
    let existing: Resource = sqlx::query_as("SELECT * FROM resources WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut title: Option<String> = None;
    let mut provider: Option<String> = None;
    let mut cover_image: Option<String> = None;
    let mut notion_url: Option<Option<String>> = None;
    let mut instructor_name: Option<String> = None;
    let mut instructor_image: Option<Option<String>> = None;
    let mut visible: Option<bool> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
    {
        let field_name = field.name().unwrap_or("").to_string();

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
                notion_url = Some(if text.is_empty() { None } else { Some(text) });
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
                    instructor_image = Some(Some(url));
                }
            }
            _ => {}
        }
    }

    let title = title.unwrap_or(existing.title);
    let provider = provider.unwrap_or(existing.provider);
    let cover_image = cover_image.or(existing.cover_image);
    let notion_url = notion_url.unwrap_or(existing.notion_url);
    let instructor_name = instructor_name.unwrap_or(existing.instructor_name);
    let instructor_image = instructor_image.unwrap_or(existing.instructor_image);
    let visible = visible.unwrap_or(existing.visible);

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
