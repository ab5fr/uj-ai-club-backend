use axum::{Json, extract::State, http::HeaderMap};
use uuid::Uuid;

use crate::{
    AppState,
    auth::extract_bearer_from_headers,
    error::AppError,
    firebase,
    models::*,
};

pub async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, AppError> {
    let token = extract_bearer_from_headers(&headers)?;

    let claims = firebase::verify_id_token(
        &state.jwk_cache,
        &state.firebase_project_id,
        &token,
    )
    .await?;

    let user = get_or_create_user(&state, &claims).await?;
    ensure_user_stats(&state, user.id).await?;

    let needs_profile: (bool,) =
        sqlx::query_as("SELECT university_major_set FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| AppError::InternalError(e.into()))?;

    Ok(Json(SessionResponse {
        user: UserResponse {
            id: user.id,
            full_name: user.full_name,
            email: user.email,
            image: user.image,
            role: user.role,
        },
        needs_profile_completion: !needs_profile.0,
    }))
}

async fn get_or_create_user(
    state: &AppState,
    claims: &firebase::FirebaseClaims,
) -> Result<User, AppError> {
    if let Some(user) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE firebase_uid = $1")
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
    {
        return Ok(user);
    }

    if claims.sign_in_provider != "google.com" {
        return Err(AppError::AuthError);
    }

    let user_id = Uuid::new_v4();
    let full_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.email.clone());

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, firebase_uid, email, full_name, image, created_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&claims.sub)
    .bind(&claims.email)
    .bind(&full_name)
    .bind(&claims.picture)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| AppError::InternalError(e.into()))?;

    Ok(user)
}

async fn ensure_user_stats(state: &AppState, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO user_stats (user_id, created_at, updated_at)
        VALUES ($1, NOW(), NOW())
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::InternalError(e.into()))?;

    Ok(())
}
