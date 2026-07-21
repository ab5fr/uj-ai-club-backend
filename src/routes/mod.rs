use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};

use crate::{AppState, handlers};

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health_routes())
        .merge(auth_routes())
        .merge(public_routes())
        .merge(challenge_routes())
        .merge(user_routes())
        .merge(webhook_routes())
        .merge(internal_routes())
        .merge(admin_routes())
}

fn internal_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/internal/jupyterhub/validate-token",
            post(handlers::validate_jupyterhub_token),
        )
        .route(
            "/internal/jupyterhub/validate-spawn",
            post(handlers::validate_jupyterhub_spawn),
        )
}

fn health_routes() -> Router<AppState> {
    Router::new().route("/health", get(handlers::health_check))
}

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/session", post(handlers::session))
        .route("/auth/complete-profile", post(handlers::complete_profile))
}

fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/leaderboards", get(handlers::get_leaderboards))
        .route("/articles", get(handlers::get_articles))
        .route("/articles/{slug}", get(handlers::get_article_by_slug))
        .route("/contact", post(handlers::create_contact))
}

fn challenge_routes() -> Router<AppState> {
    Router::new()
        .route("/challenges", get(handlers::get_challenges_with_notebooks))
        .route("/challenges/current", get(handlers::get_current_challenge))
        .route(
            "/challenges/leaderboard",
            get(handlers::get_challenge_leaderboard),
        )
        .route(
            "/challenges/{id}/leaderboard",
            get(handlers::get_challenge_submission_leaderboard),
        )
        .route(
            "/challenges/{id}/submission",
            get(handlers::get_user_submission),
        )
        .route("/challenges/{id}/start", post(handlers::start_challenge))
        .route("/challenges/{id}/submit", post(handlers::submit_challenge))
        .route(
            "/challenges/{id}/close-session",
            post(handlers::close_challenge_session),
        )
}

fn user_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/users/profile",
            put(handlers::update_user_profile).get(handlers::get_user_profile),
        )
        .route("/users/avatar", post(handlers::upload_user_avatar))
}

fn webhook_routes() -> Router<AppState> {
    Router::new().route(
        "/webhooks/nbgrader/grade",
        post(handlers::nbgrader_grade_webhook),
    )
}

fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/articles", get(handlers::admin_get_articles))
        .route(
            "/admin/articles",
            post(handlers::admin_create_article_multipart),
        )
        .route(
            "/admin/articles/{id}",
            get(handlers::admin_get_article_by_id),
        )
        .route(
            "/admin/articles/{id}",
            put(handlers::admin_update_article_multipart),
        )
        .route(
            "/admin/articles/{id}",
            delete(handlers::admin_delete_article),
        )
        .route(
            "/admin/articles/{id}/visibility",
            patch(handlers::admin_patch_article_visibility),
        )
        .route("/admin/challenges", get(handlers::admin_get_challenges))
        .route("/admin/challenges", post(handlers::admin_create_challenge))
        .route(
            "/admin/challenges/{id}",
            get(handlers::admin_get_challenge_by_id),
        )
        .route(
            "/admin/challenges/{id}",
            put(handlers::admin_update_challenge),
        )
        .route(
            "/admin/challenges/{id}",
            delete(handlers::admin_delete_challenge),
        )
        .route(
            "/admin/challenges/{id}/visibility",
            patch(handlers::admin_patch_challenge_visibility),
        )
        .route(
            "/admin/challenges/{id}/notebook",
            get(handlers::admin_get_notebook_by_challenge),
        )
        .route("/admin/notebooks", get(handlers::admin_get_notebooks))
        .route(
            "/admin/notebooks",
            post(handlers::admin_create_notebook_multipart),
        )
        .route(
            "/admin/notebooks/{id}",
            put(handlers::admin_update_notebook),
        )
        .route(
            "/admin/notebooks/{id}",
            delete(handlers::admin_delete_notebook),
        )
        .route("/admin/submissions", get(handlers::admin_get_submissions))
        .route(
            "/admin/submissions/{id}/access",
            get(handlers::admin_get_submission_access),
        )
        .route(
            "/admin/submissions/{id}/file",
            get(handlers::admin_get_submission_file),
        )
        .route(
            "/admin/submissions/{id}/grade",
            post(handlers::admin_grade_submission),
        )
        .route(
            "/admin/submissions/{id}",
            delete(handlers::admin_delete_submission),
        )
        .route(
            "/admin/submissions/{id}/grant-attempts",
            post(handlers::admin_grant_attempts),
        )
        .route(
            "/admin/contact-messages",
            get(handlers::admin_get_contact_messages),
        )
}
