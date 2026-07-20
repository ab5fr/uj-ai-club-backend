pub mod auth;
pub mod error;
pub mod firebase;
pub mod grading;
pub mod jupyterhub;
pub mod session_expiry;
pub mod submissions;
#[path = "handlers/mod.rs"]
pub mod handlers;
pub mod models;
pub mod routes;
pub mod security;

use axum::Router;
use firebase::JwkCache;
use http::HeaderValue;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::ServeDir;

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
        // Public uploads only — notebooks with solutions are never served statically.
        .nest_service("/uploads/avatars", ServeDir::new("uploads/avatars"))
        .nest_service("/uploads/resources", ServeDir::new("uploads/resources"))
        .nest_service("/uploads/certificates", ServeDir::new("uploads/certificates"))
        .nest_service("/uploads/articles", ServeDir::new("uploads/articles"))
        .layer(cors)
        .with_state(app_state)
}

fn build_cors_layer() -> CorsLayer {
    let allowed = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let origins: Vec<HeaderValue> = allowed
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    if origins.is_empty() {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any)
}
