use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, HeaderMap},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;
use time::OffsetDateTime;
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


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JupyterHubClaims {
    pub sub: String,
    pub username: String,
    pub exp: i64,
    pub iat: i64,
    pub purpose: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submission_id: Option<String>,
}

pub fn decode_jupyterhub_token(token: &str) -> Result<JupyterHubClaims, AppError> {
    let token_data = decode::<JupyterHubClaims>(token, &KEYS.decoding, &Validation::default())
        .map_err(|_| AppError::AuthError)?;
    Ok(token_data.claims)
}

pub fn create_jupyterhub_session_token(
    user_id: Uuid,
    jupyterhub_username: &str,
    submission_id: Uuid,
    jti: &str,
    expires_at: OffsetDateTime,
) -> Result<String, AppError> {
    let now = OffsetDateTime::now_utc();
    let claims = JupyterHubClaims {
        sub: user_id.to_string(),
        username: jupyterhub_username.to_string(),
        exp: expires_at.unix_timestamp(),
        iat: now.unix_timestamp(),
        purpose: "jupyterhub_sso".to_string(),
        jti: Some(jti.to_string()),
        submission_id: Some(submission_id.to_string()),
    };

    encode(&Header::default(), &claims, &KEYS.encoding).map_err(|e| AppError::InternalError(e.into()))
}

pub fn create_jupyterhub_admin_token(
    user_id: Uuid,
    admin_username: &str,
) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = JupyterHubClaims {
        sub: user_id.to_string(),
        username: admin_username.to_string(),
        exp: (now + chrono::Duration::hours(1)).timestamp(),
        iat: now.timestamp(),
        purpose: "jupyterhub_admin".to_string(),
        jti: None,
        submission_id: None,
    };

    encode(&Header::default(), &claims, &KEYS.encoding).map_err(|e| AppError::InternalError(e.into()))
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
