use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{AppState, auth::AdminUser, error::AppError, models::*};

#[derive(Deserialize)]
pub struct AdminCertificateQuery {
    #[serde(rename = "includeHidden")]
    include_hidden: Option<bool>,
}

pub async fn admin_get_certificates(
    _auth: AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminCertificateQuery>,
) -> Result<Json<AdminItemsResponse<AdminCertificateResponse>>, AppError> {
    let include_hidden = query.include_hidden.unwrap_or(false);

    let sql = if include_hidden {
        "SELECT * FROM certificates ORDER BY id"
    } else {
        "SELECT * FROM certificates WHERE visible = true ORDER BY id"
    };

    let certificates: Vec<Certificate> = sqlx::query_as(sql).fetch_all(&state.pool).await?;

    let responses: Vec<AdminCertificateResponse> = certificates
        .into_iter()
        .map(|c| AdminCertificateResponse {
            id: c.id,
            level: c.level,
            title: c.title,
            course_title: c.course_title,
            cover_image: c.cover_image,
            first_name: c.first_name,
            second_name: c.second_name,
            coursera_url: c.coursera_url,
            youtube_url: c.youtube_url,
            visible: c.visible,
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .collect();

    Ok(Json(AdminItemsResponse { items: responses }))
}
