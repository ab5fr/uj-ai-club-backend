use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    handlers::admin::upload::save_uploaded_file,
    handlers::articles::slug::{slugify, to_admin_article},
    models::*,
};

pub async fn admin_create_article_multipart(
    _auth: AdminUser,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<AdminItemResponse<AdminArticleResponse>>, AppError> {
    let mut title: Option<String> = None;
    let mut slug: Option<String> = None;
    let mut excerpt: Option<String> = None;
    let mut body: Option<String> = None;
    let mut cover_image: Option<String> = None;
    let mut visible: Option<bool> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::InternalError(e.into())
    })? {
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
            "slug" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                if !text.trim().is_empty() {
                    slug = Some(slugify(&text));
                }
            }
            "excerpt" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                if !text.trim().is_empty() {
                    excerpt = Some(text);
                }
            }
            "body" => {
                body = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?,
                );
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
                        save_uploaded_file("coverImage", &file_name, &data, "articles/covers")
                            .await?;
                    cover_image = Some(url);
                }
            }
            _ => {}
        }
    }

    let title =
        title.ok_or_else(|| AppError::BadRequest("Missing required field: title".to_string()))?;
    let body =
        body.ok_or_else(|| AppError::BadRequest("Missing required field: body".to_string()))?;
    if body.trim().is_empty() {
        return Err(AppError::BadRequest("Body cannot be empty".to_string()));
    }

    let slug = slug.unwrap_or_else(|| slugify(&title));
    let visible = visible.unwrap_or(true);

    let existing: Option<(i32,)> =
        sqlx::query_as("SELECT id FROM articles WHERE slug = $1 LIMIT 1")
            .bind(&slug)
            .fetch_optional(&state.pool)
            .await?;
    if existing.is_some() {
        return Err(AppError::BadRequest(format!(
            "An article with slug '{slug}' already exists"
        )));
    }

    let article: Article = sqlx::query_as(
        r#"
        INSERT INTO articles (title, slug, excerpt, body, cover_image, visible, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(&title)
    .bind(&slug)
    .bind(&excerpt)
    .bind(&body)
    .bind(&cover_image)
    .bind(visible)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(AdminItemResponse {
        item: to_admin_article(article),
    }))
}
