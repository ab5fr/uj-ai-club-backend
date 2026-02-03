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
pub struct AdminChallengeQuery {
    #[serde(rename = "includeHidden")]
    include_hidden: Option<bool>,
}

pub async fn admin_get_challenges(
    _auth: AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminChallengeQuery>,
) -> Result<Json<AdminItemsResponse<AdminChallengeResponse>>, AppError> {
    let include_hidden = query.include_hidden.unwrap_or(false);

    let sql = if include_hidden {
        "SELECT * FROM challenges ORDER BY id"
    } else {
        "SELECT * FROM challenges WHERE visible = true ORDER BY id"
    };

    let challenges: Vec<Challenge> = sqlx::query_as(sql).fetch_all(&state.pool).await?;

    let responses: Vec<AdminChallengeResponse> = challenges
        .into_iter()
        .map(|c| AdminChallengeResponse {
            id: c.id,
            title: c.title,
            description: c.description,
            start_date: c.start_date,
            end_date: c.end_date,
            visible: c.visible,
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .collect();

    Ok(Json(AdminItemsResponse { items: responses }))
}
