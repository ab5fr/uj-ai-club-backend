use axum::{
    extract::{FromRef, FromRequestParts},
    http::{HeaderMap, header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

use crate::{AppState, error::AppError, firebase};

static KEYS: Lazy<Keys> = Lazy::new(|| {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    Keys::new(secret.as_bytes())
});

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

pub fn extract_bearer_from_headers(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get(AUTHORIZATION)
        .ok_or(AppError::AuthError)?
        .to_str()
        .map_err(|_| AppError::AuthError)?
        .strip_prefix("Bearer ")
        .ok_or(AppError::AuthError)
        .map(str::to_string)
}

pub fn extract_bearer_token(parts: &Parts) -> Result<String, AppError> {
    extract_bearer_from_headers(&parts.headers)
}

/// Claims for JupyterHub SSO token
#[derive(Debug, Serialize, Deserialize)]
pub struct JupyterHubClaims {
    pub sub: String,      // User ID
    pub username: String, // JupyterHub username
    pub exp: i64,         // Expiration
    pub iat: i64,         // Issued at
    pub purpose: String,  // "jupyterhub_sso"
}

/// Create a JWT token for JupyterHub SSO authentication
/// This token has a shorter lifespan (1 hour) and includes the JupyterHub username
pub fn create_jupyterhub_token(
    user_id: Uuid,
    jupyterhub_username: &str,
) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = JupyterHubClaims {
        sub: user_id.to_string(),
        username: jupyterhub_username.to_string(),
        exp: (now + chrono::Duration::hours(1)).timestamp(),
        iat: now.timestamp(),
        purpose: "jupyterhub_sso".to_string(),
    };

    encode(&Header::default(), &claims, &KEYS.encoding)
        .map_err(|e| AppError::InternalError(e.into()))
}

/// Verify and decode a JupyterHub SSO token
/// Returns the claims if valid
pub fn verify_jupyterhub_token(token: &str) -> Result<JupyterHubClaims, AppError> {
    let token_data = decode::<JupyterHubClaims>(token, &KEYS.decoding, &Validation::default())
        .map_err(|_| AppError::AuthError)?;

    if token_data.claims.purpose != "jupyterhub_sso" {
        return Err(AppError::AuthError);
    }

    Ok(token_data.claims)
}

async fn resolve_user_id(state: &AppState, token: &str) -> Result<Uuid, AppError> {
    let claims = firebase::verify_id_token(
        &state.jwk_cache,
        &state.firebase_project_id,
        token,
    )
    .await?;

    let user_id: (Uuid,) = sqlx::query_as("SELECT id FROM users WHERE firebase_uid = $1")
        .bind(&claims.sub)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or(AppError::AuthError)?;

    Ok(user_id.0)
}

pub struct AuthUser {
    pub user_id: Uuid,
}

pub struct AdminUser {
    pub user_id: Uuid,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_bearer_token(parts)?;
        let user_id = resolve_user_id(&app_state, &token).await?;

        Ok(Self { user_id })
    }
}

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_bearer_token(parts)?;
        let user_id = resolve_user_id(&app_state, &token).await?;

        let user_role: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&app_state.pool)
            .await
            .map_err(|e| AppError::InternalError(e.into()))?
            .ok_or(AppError::AuthError)?;

        if user_role.0 != "admin" {
            return Err(AppError::Forbidden);
        }

        Ok(Self { user_id })
    }
}
