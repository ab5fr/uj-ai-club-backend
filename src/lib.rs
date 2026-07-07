pub mod auth;
pub mod error;
pub mod firebase;
#[path = "handlers/mod.rs"]
pub mod handlers;
pub mod models;
pub mod routes;

use axum::Router;
use firebase::JwkCache;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
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

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    routes::api_routes()
        .nest_service("/uploads", ServeDir::new("uploads"))
        .layer(cors)
        .with_state(app_state)
}
