use axum::{Json, extract::State};

use crate::{AppState, error::AppError, models::*};

pub async fn get_articles(
    State(state): State<AppState>,
) -> Result<Json<Vec<ArticleListResponse>>, AppError> {
    let articles: Vec<Article> = sqlx::query_as(
        "SELECT * FROM articles WHERE visible = true ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    let responses: Vec<ArticleListResponse> = articles
        .into_iter()
        .map(|a| ArticleListResponse {
            id: a.id,
            title: a.title,
            slug: a.slug,
            excerpt: a.excerpt,
            cover_image: a.cover_image,
            created_at: a.created_at,
        })
        .collect();

    Ok(Json(responses))
}
