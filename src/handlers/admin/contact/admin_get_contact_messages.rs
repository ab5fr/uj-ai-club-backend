use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

pub async fn admin_get_contact_messages(
    _auth: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<AdminItemsResponse<AdminContactMessageResponse>>, AppError> {
    let messages: Vec<ContactMessage> = sqlx::query_as(
        "SELECT id, name, email, message, sender_ip, created_at FROM contact_messages ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    let items = messages
        .into_iter()
        .map(|message| AdminContactMessageResponse {
            id: message.id,
            name: message.name,
            email: message.email,
            message: message.message,
            created_at: message.created_at,
        })
        .collect();

    Ok(Json(AdminItemsResponse { items }))
}
