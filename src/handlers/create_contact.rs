use axum::{Json, extract::State};

use crate::{AppState, error::AppError, models::*};

pub async fn create_contact(
    State(state): State<AppState>,
    Json(req): Json<ContactRequest>,
) -> Result<Json<ContactResponse>, AppError> {
    sqlx::query(
        "INSERT INTO contact_messages (name, email, message, created_at) VALUES ($1, $2, $3, NOW())",
    )
    .bind(req.name)
    .bind(req.email)
    .bind(req.message)
    .execute(&state.pool)
    .await?;

    Ok(Json(ContactResponse {
        success: true,
        message: "Message sent successfully".to_string(),
    }))
}
