pub mod auth;
pub mod error;
pub mod firebase;
pub mod grading;
#[path = "handlers/mod.rs"]
pub mod handlers;
pub mod jupyterhub;
pub mod models;
pub mod routes;
pub mod security;
pub mod session_expiry;
pub mod submissions;

use axum::Router;
use firebase::JwkCache;
use http::header::{AUTHORIZATION, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
use http::{HeaderValue, Method};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub firebase_project_id: String,
    pub jwk_cache: Arc<RwLock<JwkCache>>,
}

impl axum::extract::FromRef<AppState> for sqlx::PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

pub fn create_app(pool: sqlx::PgPool) -> Router {
    let firebase_project_id =
        std::env::var("FIREBASE_PROJECT_ID").expect("FIREBASE_PROJECT_ID must be set");

    let app_state = AppState {
        pool: pool.clone(),
        firebase_project_id,
        jwk_cache: Arc::new(RwLock::new(JwkCache::new())),
    };

    let cors = build_cors_layer();

    routes::api_routes()
        .nest_service("/uploads/avatars", ServeDir::new("uploads/avatars"))
        .nest_service("/uploads/articles", ServeDir::new("uploads/articles"))
        // Prevent browsers from MIME-sniffing uploaded files (validated by
        // extension only) into an executable content type such as HTML.
        .layer(SetResponseHeaderLayer::overriding(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(cors)
        .with_state(app_state)
}

fn build_cors_layer() -> CorsLayer {
    const DEFAULT_ORIGIN: &str = "http://localhost:3000";

    let allowed =
        std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| DEFAULT_ORIGIN.to_string());

    let origins: Vec<HeaderValue> = allowed
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    // Never fall back to allowing any origin: an empty/invalid configuration
    // should degrade to the safe default rather than reflecting every origin.
    let origins = if origins.is_empty() {
        tracing::warn!(
            "CORS_ALLOWED_ORIGINS did not yield any valid origins; \
             falling back to {DEFAULT_ORIGIN} instead of allowing all origins"
        );
        vec![HeaderValue::from_static(DEFAULT_ORIGIN)]
    } else {
        origins
    };

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
}
