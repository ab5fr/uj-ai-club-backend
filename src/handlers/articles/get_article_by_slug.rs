use axum::{
    Json,
    extract::{Path, State},
};

use crate::{AppState, error::AppError, models::*};

pub async fn get_article_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<ArticleDetailResponse>, AppError> {
    let article: Article =
        sqlx::query_as("SELECT * FROM articles WHERE slug = $1 AND visible = true")
            .bind(&slug)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    Ok(Json(ArticleDetailResponse {
        id: article.id,
        title: article.title,
        slug: article.slug,
        excerpt: article.excerpt,
        body: article.body,
        cover_image: article.cover_image,
        created_at: article.created_at,
        updated_at: article.updated_at,
    }))
}
