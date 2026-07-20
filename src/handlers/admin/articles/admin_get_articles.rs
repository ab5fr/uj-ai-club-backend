use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    handlers::articles::slug::to_admin_article,
    models::*,
};

#[derive(Deserialize)]
pub struct AdminArticleQuery {
    #[serde(rename = "includeHidden")]
    include_hidden: Option<bool>,
}

pub async fn admin_get_articles(
    _auth: AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminArticleQuery>,
) -> Result<Json<AdminItemsResponse<AdminArticleResponse>>, AppError> {
    let include_hidden = query.include_hidden.unwrap_or(false);

    let sql = if include_hidden {
        "SELECT * FROM articles ORDER BY created_at DESC"
    } else {
        "SELECT * FROM articles WHERE visible = true ORDER BY created_at DESC"
    };

    let articles: Vec<Article> = sqlx::query_as(sql).fetch_all(&state.pool).await?;

    let responses: Vec<AdminArticleResponse> =
        articles.into_iter().map(to_admin_article).collect();

    Ok(Json(AdminItemsResponse { items: responses }))
}
