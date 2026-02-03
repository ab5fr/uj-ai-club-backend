use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::AdminUser,
    error::AppError,
    models::*,
};

/// Admin access to JupyterHub for editing notebooks
/// Returns a JupyterHub URL where admin can edit the source notebook with grading cells
pub async fn admin_get_notebook_edit_url(
    auth: AdminUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<i32>,
) -> Result<Json<AdminJupyterHubAccessResponse>, AppError> {
    // Get the notebook
    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE id = $1")
            .bind(notebook_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    // Get admin user info
    let admin: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    // Generate admin JupyterHub username
    let admin_jupyterhub_username = format!("admin_{}", auth.user_id.to_string().replace("-", ""));

    // Update admin's jupyterhub_username if not set
    sqlx::query(
        "UPDATE users SET jupyterhub_username = $1 WHERE id = $2 AND jupyterhub_username IS NULL",
    )
    .bind(&admin_jupyterhub_username)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await?;

    // Create JWT token for JupyterHub SSO
    let jupyterhub_token =
        crate::auth::create_jupyterhub_token(auth.user_id, &admin_jupyterhub_username)?;

    // Get the notebook filename (strip UUID prefix if present)
    let notebook_filename = &notebook.notebook_filename;

    // Generate JupyterHub URL
    let jupyterhub_base_url =
        std::env::var("JUPYTERHUB_URL").unwrap_or_else(|_| "http://localhost:8888".to_string());

    // For admin editing, we want to open the notebook directly in their workspace
    // The notebook will be copied to their workspace when they spawn
    // URL pattern: login -> spawn -> notebook
    let next_path = format!(
        "/user/{}/notebooks/{}",
        admin_jupyterhub_username, notebook_filename
    );
    let encoded_next = urlencoding::encode(&next_path);
    let jupyterhub_url = format!(
        "{}/hub/login?token={}&next=/hub/spawn/{}?next={}",
        jupyterhub_base_url, jupyterhub_token, admin_jupyterhub_username, encoded_next
    );

    tracing::info!(
        "Admin {} accessing notebook {} for editing",
        admin.email,
        notebook.assignment_name
    );

    Ok(Json(AdminJupyterHubAccessResponse {
        success: true,
        jupyterhub_url,
        token: jupyterhub_token,
        message: format!(
            "Opening notebook '{}' in JupyterHub. Add grading cells with ### BEGIN SOLUTION / ### END SOLUTION markers.",
            notebook_filename
        ),
    }))
}
