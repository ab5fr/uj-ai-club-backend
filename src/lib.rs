pub mod auth;
pub mod error;
pub mod firebase;
#[path = "handlers/mod.rs"]
pub mod handlers;
pub mod models;

use axum::{
    Router,
    extract::FromRef,
    routing::{delete, get, patch, post, put},
};
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

impl FromRef<AppState> for sqlx::PgPool {
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

    Router::new()
        // Health
        .route("/health", get(handlers::health_check))
        // Auth
        .route("/auth/session", post(handlers::session))
        .route("/auth/complete-profile", post(handlers::complete_profile))
        // Public content
        .route("/leaderboards", get(handlers::get_leaderboards))
        .route("/resources", get(handlers::get_resources))
        .route("/resources/:id", get(handlers::get_resource_by_id))
        .route("/certificates", get(handlers::get_certificates))
        .route("/certificates/:id", get(handlers::get_certificate_by_id))
        .route("/contact", post(handlers::create_contact))
        // Challenges
        .route("/challenges", get(handlers::get_challenges_with_notebooks))
        .route("/challenges/current", get(handlers::get_current_challenge))
        .route(
            "/challenges/leaderboard",
            get(handlers::get_challenge_leaderboard),
        )
        .route(
            "/challenges/:id/leaderboard",
            get(handlers::get_challenge_submission_leaderboard),
        )
        .route(
            "/challenges/:id/submission",
            get(handlers::get_user_submission),
        )
        .route("/challenges/:id/start", post(handlers::start_challenge))
        .route("/challenges/:id/submit", post(handlers::submit_challenge))
        // Users
        .route(
            "/users/profile",
            put(handlers::update_user_profile).get(handlers::get_user_profile),
        )
        .route("/users/avatar", post(handlers::upload_user_avatar))
        // Webhooks
        .route(
            "/webhooks/nbgrader/grade",
            post(handlers::nbgrader_grade_webhook),
        )
        // Admin: resources
        .route("/admin/resources", get(handlers::admin_get_resources))
        .route(
            "/admin/resources",
            post(handlers::admin_create_resource_multipart),
        )
        .route(
            "/admin/resources/:id",
            get(handlers::admin_get_resource_by_id),
        )
        .route(
            "/admin/resources/:id",
            put(handlers::admin_update_resource_multipart),
        )
        .route(
            "/admin/resources/:id",
            delete(handlers::admin_delete_resource),
        )
        .route(
            "/admin/resources/:id/visibility",
            patch(handlers::admin_patch_resource_visibility),
        )
        // Admin: certificates
        .route("/admin/certificates", get(handlers::admin_get_certificates))
        .route(
            "/admin/certificates",
            post(handlers::admin_create_certificate_multipart),
        )
        .route(
            "/admin/certificates/:id",
            get(handlers::admin_get_certificate_by_id),
        )
        .route(
            "/admin/certificates/:id",
            put(handlers::admin_update_certificate_multipart),
        )
        .route(
            "/admin/certificates/:id",
            delete(handlers::admin_delete_certificate),
        )
        .route(
            "/admin/certificates/:id/visibility",
            patch(handlers::admin_patch_certificate_visibility),
        )
        // Admin: challenges
        .route("/admin/challenges", get(handlers::admin_get_challenges))
        .route("/admin/challenges", post(handlers::admin_create_challenge))
        .route(
            "/admin/challenges/:id",
            get(handlers::admin_get_challenge_by_id),
        )
        .route(
            "/admin/challenges/:id",
            put(handlers::admin_update_challenge),
        )
        .route(
            "/admin/challenges/:id",
            delete(handlers::admin_delete_challenge),
        )
        .route(
            "/admin/challenges/:id/visibility",
            patch(handlers::admin_patch_challenge_visibility),
        )
        .route(
            "/admin/challenges/:id/notebook",
            get(handlers::admin_get_notebook_by_challenge),
        )
        // Admin: notebooks
        .route("/admin/notebooks", get(handlers::admin_get_notebooks))
        .route(
            "/admin/notebooks",
            post(handlers::admin_create_notebook_multipart),
        )
        .route("/admin/notebooks/:id", put(handlers::admin_update_notebook))
        .route(
            "/admin/notebooks/:id",
            delete(handlers::admin_delete_notebook),
        )
        .route(
            "/admin/notebooks/:id/edit",
            get(handlers::admin_get_notebook_edit_url),
        )
        .route(
            "/admin/notebooks/:id/sync",
            post(handlers::admin_sync_notebook_to_nbgrader),
        )
        // Admin: submissions
        .route("/admin/submissions", get(handlers::admin_get_submissions))
        .route(
            "/admin/submissions/:id/access",
            get(handlers::admin_get_submission_access),
        )
        .route(
            "/admin/submissions/:id/file",
            get(handlers::admin_get_submission_file),
        )
        .route(
            "/admin/submissions/:id/grade",
            post(handlers::admin_grade_submission),
        )
        .route(
            "/admin/contact-messages",
            get(handlers::admin_get_contact_messages),
        )
        // Static
        .nest_service("/uploads", ServeDir::new("uploads"))
        .layer(cors)
        .with_state(app_state)
}
