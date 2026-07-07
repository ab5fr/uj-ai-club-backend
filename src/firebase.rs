use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::error::AppError;

const JWK_URL: &str =
    "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com";
const JWK_CACHE_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone)]
pub struct FirebaseClaims {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub sign_in_provider: String,
}

#[derive(Debug, Deserialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Debug, Clone)]
struct CachedJwk {
    n: String,
    e: String,
}

pub struct JwkCache {
    keys: HashMap<String, CachedJwk>,
    fetched_at: Option<Instant>,
}

impl Default for JwkCache {
    fn default() -> Self {
        Self {
            keys: HashMap::new(),
            fetched_at: None,
        }
    }
}

impl JwkCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_stale(&self) -> bool {
        self.fetched_at
            .map(|t| t.elapsed() > JWK_CACHE_TTL)
            .unwrap_or(true)
    }
}

#[derive(Debug, Deserialize)]
struct TokenClaims {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
    firebase: FirebaseBlock,
}

#[derive(Debug, Deserialize)]
struct FirebaseBlock {
    #[serde(rename = "sign_in_provider")]
    sign_in_provider: String,
}

pub async fn verify_id_token(
    cache: &Arc<RwLock<JwkCache>>,
    project_id: &str,
    token: &str,
) -> Result<FirebaseClaims, AppError> {
    let header = decode_header(token).map_err(|_| AppError::AuthError)?;
    let kid = header.kid.ok_or(AppError::AuthError)?;

    let decoding_key = get_decoding_key(cache, &kid).await?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[project_id]);
    validation.set_issuer(&[&format!("https://securetoken.google.com/{project_id}")]);

    let token_data = decode::<TokenClaims>(token, &decoding_key, &validation)
        .map_err(|_| AppError::AuthError)?;

    let email = token_data.claims.email.ok_or(AppError::AuthError)?;

    Ok(FirebaseClaims {
        sub: token_data.claims.sub,
        email,
        name: token_data.claims.name,
        picture: token_data.claims.picture,
        sign_in_provider: token_data.claims.firebase.sign_in_provider,
    })
}

async fn get_decoding_key(
    cache: &Arc<RwLock<JwkCache>>,
    kid: &str,
) -> Result<DecodingKey, AppError> {
    {
        let cache_read = cache.read().await;
        if !cache_read.is_stale()
            && let Some(jwk) = cache_read.keys.get(kid)
        {
            return decoding_key_from_jwk(jwk);
        }
    }

    refresh_jwks(cache).await?;

    {
        let cache_read = cache.read().await;
        if let Some(jwk) = cache_read.keys.get(kid) {
            return decoding_key_from_jwk(jwk);
        }
    }

    Err(AppError::AuthError)
}

async fn refresh_jwks(cache: &Arc<RwLock<JwkCache>>) -> Result<(), AppError> {
    let jwk_set: JwkSet = reqwest::get(JWK_URL)
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
        .json()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut keys = HashMap::new();
    for jwk in jwk_set.keys {
        keys.insert(jwk.kid.clone(), CachedJwk { n: jwk.n, e: jwk.e });
    }

    let mut cache_write = cache.write().await;
    cache_write.keys = keys;
    cache_write.fetched_at = Some(Instant::now());

    Ok(())
}

fn decoding_key_from_jwk(jwk: &CachedJwk) -> Result<DecodingKey, AppError> {
    DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|_| AppError::AuthError)
}
