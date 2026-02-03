use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

#[derive(Deserialize)]
pub struct AdminResourceQuery {
    #[serde(rename = "includeHidden")]
    include_hidden: Option<bool>,
}

pub async fn admin_get_resources(
    _auth: AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminResourceQuery>,
) -> Result<Json<AdminItemsResponse<AdminResourceResponse>>, AppError> {
    let include_hidden = query.include_hidden.unwrap_or(false);

    let sql = if include_hidden {
        "SELECT * FROM resources ORDER BY id"
    } else {
        "SELECT * FROM resources WHERE visible = true ORDER BY id"
    };

    let resources: Vec<Resource> = sqlx::query_as(sql).fetch_all(&state.pool).await?;

    let responses: Vec<AdminResourceResponse> = resources
        .into_iter()
        .map(|r| AdminResourceResponse {
            id: r.id,
            title: r.title,
            provider: r.provider,
            cover_image: r.cover_image,
            notion_url: r.notion_url,
            instructor: Some(AdminInstructorResponse {
                name: r.instructor_name,
                image: r.instructor_image,
            }),
            quote: None, // Quotes are now in a separate table
            visible: r.visible,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect();

    Ok(Json(AdminItemsResponse { items: responses }))
}
