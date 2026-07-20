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

pub async fn admin_patch_article_visibility(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<AdminVisibilityRequest>,
) -> Result<Json<AdminItemResponse<AdminArticleResponse>>, AppError> {
    let article: Article = sqlx::query_as(
        "UPDATE articles SET visible = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
    )
    .bind(req.visible)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(AdminItemResponse {
        item: to_admin_article(article),
    }))
}
