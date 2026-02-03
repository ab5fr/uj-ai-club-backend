use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    auth::create_token,
    error::AppError,
    models::*,
};

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    code: String,
    #[allow(dead_code)]
    state: String,
}

pub async fn google_auth_callback(
    State(state): State<AppState>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    use oauth2::basic::BasicClient;
    use oauth2::{
        AuthUrl, AuthorizationCode, ClientId, ClientSecret, RedirectUrl, TokenResponse, TokenUrl,
    };

    // Create OAuth client
    let client = BasicClient::new(
        ClientId::new(state.oauth_config.client_id.clone()),
        Some(ClientSecret::new(state.oauth_config.client_secret.clone())),
        AuthUrl::new(state.oauth_config.auth_url.clone())
            .expect("Invalid authorization endpoint URL"),
        Some(
            TokenUrl::new(state.oauth_config.token_url.clone())
                .expect("Invalid token endpoint URL"),
        ),
    )
    .set_redirect_uri(
        RedirectUrl::new(state.oauth_config.redirect_uri.clone()).expect("Invalid redirect URL"),
    );

    // Exchange authorization code for access token
    let token_result = client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token exchange failed: {e}")))?;

    // Fetch user info from Google
    let user_info: GoogleUserInfo = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(token_result.access_token().secret())
        .send()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
        .json()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?;

    // Check if user exists with this google_id
    let existing_user: Option<User> = sqlx::query_as(
        "SELECT id, email, password_hash, full_name, phone_num, image, points, rank, role, jupyterhub_username, created_at \
         FROM users WHERE google_id = $1"
    )
    .bind(&user_info.sub)
    .fetch_optional(&state.pool)
    .await?;

    let user = if let Some(user) = existing_user {
        // User exists, update their info if needed
        sqlx::query_as(
            "UPDATE users SET email = $1, full_name = $2, image = $3 \
             WHERE google_id = $4
             RETURNING id, email, password_hash, full_name, phone_num, image, points, rank, role, jupyterhub_username, created_at"
        )
        .bind(&user_info.email)
        .bind(user_info.name.as_deref().unwrap_or(&user.full_name))
        .bind(&user_info.picture)
        .bind(&user_info.sub)
        .fetch_one(&state.pool)
        .await?
    } else {
        // Check if user exists with same email (linking accounts)
        let email_user: Option<User> = sqlx::query_as(
            "SELECT id, email, password_hash, full_name, phone_num, image, points, rank, role, jupyterhub_username, created_at \
             FROM users WHERE email = $1"
        )
        .bind(&user_info.email)
        .fetch_optional(&state.pool)
        .await?;

        if let Some(existing) = email_user {
            // Link Google account to existing user
            sqlx::query_as(
                "UPDATE users SET google_id = $1, image = COALESCE($2, image) \
                 WHERE id = $3
                 RETURNING id, email, password_hash, full_name, phone_num, image, points, rank, role, jupyterhub_username, created_at"
            )
            .bind(&user_info.sub)
            .bind(&user_info.picture)
            .bind(existing.id)
            .fetch_one(&state.pool)
            .await?
        } else {
            // Create new user
            let user_id = Uuid::new_v4();
            let user: User = sqlx::query_as(
                r#"
                INSERT INTO users (id, email, password_hash, full_name, google_id, image, created_at)
                VALUES ($1, $2, NULL, $3, $4, $5, NOW())
                RETURNING id, email, password_hash, full_name, phone_num, image, points, rank, role, jupyterhub_username, created_at
                "#,
            )
            .bind(user_id)
            .bind(&user_info.email)
            .bind(user_info.name.as_deref().unwrap_or(&user_info.email))
            .bind(&user_info.sub)
            .bind(&user_info.picture)
            .fetch_one(&state.pool)
            .await?;

            // Create user stats
            sqlx::query(
                "INSERT INTO user_stats (user_id, created_at, updated_at) VALUES ($1, NOW(), NOW())",
            )
            .bind(user_id)
            .execute(&state.pool)
            .await?;

            user
        }
    };

    // Check if user needs to complete profile (university and major)
    let needs_profile: Option<(bool,)> =
        sqlx::query_as("SELECT university_major_set FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_optional(&state.pool)
            .await?;

    let needs_completion = needs_profile.map(|(set,)| !set).unwrap_or(true);

    // Check if user needs to set a password (password_hash is NULL)
    let needs_password = user.password_hash.is_none();

    // Create JWT token
    let token = create_token(user.id)?;

    // Encode user data
    let user_json = serde_json::to_string(&UserResponse {
        id: user.id,
        full_name: user.full_name,
        email: user.email,
        image: user.image,
        role: user.role,
    })
    .map_err(|e| AppError::InternalError(e.into()))?;

    let encoded_user = urlencoding::encode(&user_json);

    // Get frontend URL from environment or use default
    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "https://aiclub-uj.com".to_string());

    // Redirect to frontend with token and user data
    // If user needs password or profile completion, redirect to complete-profile page
    let redirect_url = if needs_password || needs_completion {
        format!(
            "{frontend_url}/auth/callback?token={token}&user={encoded_user}&needs_profile_completion=true&needs_password={needs_password}"
        )
    } else {
        format!("{frontend_url}/auth/callback?token={token}&user={encoded_user}")
    };

    Ok(Redirect::temporary(&redirect_url))
}
