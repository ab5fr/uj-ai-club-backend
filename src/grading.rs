use reqwest::Client;

use crate::error::AppError;

const GRADING_SECRET_HEADER: &str = "X-Grading-Service-Secret";


pub fn grading_client() -> Result<Client, AppError> {
    Client::builder()
        .build()
        .map_err(|e| AppError::InternalError(e.into()))
}

pub fn grading_service_url() -> String {
    std::env::var("GRADING_SERVICE_URL").unwrap_or_else(|_| "http://localhost:9100".to_string())
}

pub fn grading_service_secret() -> Result<String, AppError> {
    let secret = std::env::var("GRADING_SERVICE_SECRET").map_err(|_| {
        AppError::InternalError(anyhow::anyhow!("GRADING_SERVICE_SECRET must be set"))
    })?;

    if secret.is_empty() {
        return Err(AppError::InternalError(anyhow::anyhow!(
            "GRADING_SERVICE_SECRET must not be empty"
        )));
    }

    Ok(secret)
}

/// Secret for `/internal/*` routes. Prefers `INTERNAL_SERVICE_SECRET`, falls
/// back to `GRADING_SERVICE_SECRET` for backwards compatibility.
pub fn internal_service_secret() -> Result<String, AppError> {
    if let Ok(secret) = std::env::var("INTERNAL_SERVICE_SECRET") {
        let trimmed = secret.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    grading_service_secret()
}

pub fn apply_grading_auth(
    request: reqwest::RequestBuilder,
) -> Result<reqwest::RequestBuilder, AppError> {
    let secret = grading_service_secret()?;
    Ok(request.header(GRADING_SECRET_HEADER, secret))
}


pub async fn save_user_notebook(
    jupyterhub_username: &str,
    notebook_filename: &str,
) -> Result<(), AppError> {
    let grading_service_url = grading_service_url();
    let client = grading_client()?;
    let save_url = format!("{grading_service_url}/save-notebook/{jupyterhub_username}");

    let payload = serde_json::json!({
        "notebookFilename": notebook_filename
    });

    let response = apply_grading_auth(client.post(&save_url).json(&payload))?
        .send()
        .await
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!(
                "Failed to call grading service to save notebook for {jupyterhub_username}: {e}"
            ))
        })?;

    if response.status().is_success() {
        tracing::info!("Saved notebook for {jupyterhub_username} before closing Jupyter");
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        Err(AppError::InternalError(anyhow::anyhow!(
            "Notebook save failed for {jupyterhub_username}: {status} {error_text}"
        )))
    }
}


pub async fn close_jupyter_session(
    jupyterhub_username: &str,
    notebook_filename: &str,
) -> Result<(), AppError> {
    if let Err(e) = save_user_notebook(jupyterhub_username, notebook_filename).await {
        tracing::warn!(
            "Notebook save failed for {jupyterhub_username} (continuing with shutdown): {e}"
        );
    }

    if let Err(e) = crate::jupyterhub::stop_user_server(jupyterhub_username).await {
        tracing::warn!("Failed to stop JupyterHub server for {jupyterhub_username}: {e}");
    } else {
        tracing::info!("Closed Jupyter session for {jupyterhub_username}");
    }

    Ok(())
}


pub async fn sync_notebook_to_nbgrader(
    assignment_name: &str,
    notebook_path: &str,
    max_points: i32,
) -> Result<(), crate::error::AppError> {
    let grading_service_url = grading_service_url();
    let client = grading_client()?;
    let sync_url = format!("{grading_service_url}/setup-assignment/{assignment_name}");

    let nb_path = format!("/srv/notebooks/{}", notebook_path.replace("uploads/", ""));

    let payload = serde_json::json!({
        "notebookPath": nb_path,
        "assignmentName": assignment_name,
        "maxPoints": max_points
    });

    tracing::info!("Syncing notebook {assignment_name} to nbgrader");

    let response = apply_grading_auth(client.post(&sync_url).json(&payload))?
        .send()
        .await
        .map_err(|e| {
            crate::error::AppError::InternalError(anyhow::anyhow!(
                "Failed to call grading service for nbgrader sync: {e}"
            ))
        })?;

    if response.status().is_success() {
        tracing::info!("Notebook {assignment_name} synced to nbgrader");
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        Err(crate::error::AppError::InternalError(anyhow::anyhow!(
            "nbgrader sync failed for {assignment_name}: {status} {error_text}"
        )))
    }
}


pub async fn cleanup_nbgrader_assignment(
    assignment_name: &str,
) -> Result<(), crate::error::AppError> {
    let grading_service_url = grading_service_url();
    let client = grading_client()?;
    let cleanup_url = format!("{grading_service_url}/cleanup-assignment/{assignment_name}");

    tracing::info!("Cleaning up nbgrader assignment {assignment_name}");

    let response = apply_grading_auth(client.delete(&cleanup_url))?
        .send()
        .await
        .map_err(|e| {
            crate::error::AppError::InternalError(anyhow::anyhow!(
                "Failed to call grading service for nbgrader cleanup: {e}"
            ))
        })?;

    if response.status().is_success() {
        tracing::info!("Assignment {assignment_name} cleaned up from nbgrader");
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        Err(crate::error::AppError::InternalError(anyhow::anyhow!(
            "nbgrader cleanup failed for {assignment_name}: {status} {error_text}"
        )))
    }
}
