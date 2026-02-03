use axum::{Json, extract::State};
use bcrypt::{DEFAULT_COST, hash};
use uuid::Uuid;

use crate::{
    AppState,
    auth::create_token,
    error::AppError,
    models::*,
};

pub async fn signup(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let existing_user = sqlx::query("SELECT id FROM users WHERE email = $1")
        .bind(&req.email)
        .fetch_optional(&state.pool)
        .await?;

    if existing_user.is_some() {
        return Err(AppError::UserExists);
    }

    let password_hash = hash(req.password.as_bytes(), DEFAULT_COST)
        .map_err(|e| AppError::InternalError(e.into()))?;

    let user_id = Uuid::new_v4();

    let user: User = sqlx::query_as(
        r#"
        INSERT INTO users (id, email, password_hash, full_name, phone_num, created_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        RETURNING id, email, password_hash, full_name, phone_num, image, points, rank, role, jupyterhub_username, created_at
        "#,
    )
    .bind(user_id)
    .bind(&req.email)
    .bind(Some(password_hash))
    .bind(req.full_name)
    .bind(req.phone_num)
    .fetch_one(&state.pool)
    .await?;

    sqlx::query(
        "INSERT INTO user_stats (user_id, created_at, updated_at) VALUES ($1, NOW(), NOW())",
    )
    .bind(user_id)
    .execute(&state.pool)
    .await?;

    let token = create_token(user.id)?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            full_name: user.full_name,
            email: user.email,
            image: user.image,
            role: user.role,
        },
    }))
}
