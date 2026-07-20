use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    handlers::articles::slug::to_admin_article,
    models::*,
};

pub async fn admin_get_article_by_id(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdminItemResponse<AdminArticleResponse>>, AppError> {
    let article: Article = sqlx::query_as("SELECT * FROM articles WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(AdminItemResponse {
        item: to_admin_article(article),
    }))
}
