use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    handlers::admin::upload::save_uploaded_file,
    handlers::articles::slug::{slugify, to_admin_article},
    models::*,
};

pub async fn admin_update_article_multipart(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<AdminItemResponse<AdminArticleResponse>>, AppError> {
    let existing: Article = sqlx::query_as("SELECT * FROM articles WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut title: Option<String> = None;
    let mut slug: Option<String> = None;
    let mut excerpt: Option<Option<String>> = None;
    let mut body: Option<String> = None;
    let mut cover_image: Option<String> = None;
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
                excerpt = Some(if text.trim().is_empty() {
                    None
                } else {
                    Some(text)
                });
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

    let title = title.unwrap_or(existing.title);
    let body = body.unwrap_or(existing.body);
    if body.trim().is_empty() {
        return Err(AppError::BadRequest("Body cannot be empty".to_string()));
    }
    let slug = slug.unwrap_or(existing.slug);
    let excerpt = excerpt.unwrap_or(existing.excerpt);
    let cover_image = cover_image.or(existing.cover_image);
    let visible = visible.unwrap_or(existing.visible);

    let conflict: Option<(i32,)> =
        sqlx::query_as("SELECT id FROM articles WHERE slug = $1 AND id <> $2 LIMIT 1")
            .bind(&slug)
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;
    if conflict.is_some() {
        return Err(AppError::BadRequest(format!(
            "An article with slug '{slug}' already exists"
        )));
    }

    let article: Article = sqlx::query_as(
        r#"
        UPDATE articles
        SET title = $1, slug = $2, excerpt = $3, body = $4, cover_image = $5, visible = $6, updated_at = NOW()
        WHERE id = $7
        RETURNING *
        "#,
    )
    .bind(&title)
    .bind(&slug)
    .bind(&excerpt)
    .bind(&body)
    .bind(&cover_image)
    .bind(visible)
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(AdminItemResponse {
        item: to_admin_article(article),
    }))
}
