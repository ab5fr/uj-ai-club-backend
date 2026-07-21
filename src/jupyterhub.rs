use reqwest::Client;
use serde::Deserialize;

use crate::error::AppError;


pub fn jupyterhub_public_url() -> String {
    std::env::var("JUPYTERHUB_URL").unwrap_or_else(|_| "http://localhost:8888".to_string())
}


pub fn jupyterhub_internal_api_url() -> String {
    std::env::var("JUPYTERHUB_API_URL").unwrap_or_else(|_| "http://jupyterhub:8000".to_string())
}

#[deprecated(note = "use jupyterhub_public_url for browser URLs")]
pub fn jupyterhub_api_url() -> String {
    jupyterhub_public_url()
}

pub fn jupyterhub_api_token() -> Result<String, AppError> {
    std::env::var("JUPYTERHUB_API_TOKEN")
        .map_err(|_| AppError::InternalError(anyhow::anyhow!("JUPYTERHUB_API_TOKEN must be set")))
}

pub fn student_jupyterhub_username(user_id: uuid::Uuid) -> String {
    format!("user_{}", user_id.to_string().replace('-', ""))
}

pub fn admin_jupyterhub_username(user_id: uuid::Uuid) -> String {
    format!("admin_{}", user_id.to_string().replace('-', ""))
}

pub async fn user_server_running(username: &str) -> Result<bool, AppError> {
    let token = jupyterhub_api_token()?;
    let base = jupyterhub_internal_api_url();
    let client = Client::new();
    let url = format!("{base}/hub/api/users/{username}");

    let resp = client
        .get(&url)
        .header("Authorization", format!("token {token}"))
        .send()
        .await
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!(
                "JupyterHub user lookup failed for {username}: {e}"
            ))
        })?;

    if resp.status().as_u16() == 404 {
        return Ok(false);
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::InternalError(anyhow::anyhow!(
            "JupyterHub user lookup failed for {username}: {status} {body}"
        )));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| {
        AppError::InternalError(anyhow::anyhow!(
            "Invalid JupyterHub user response for {username}: {e}"
        ))
    })?;

    let running = body
        .get("servers")
        .and_then(|servers| servers.as_object())
        .is_some_and(|servers| !servers.is_empty());

    Ok(running)
}


pub async fn stop_user_server(username: &str) -> Result<(), AppError> {
    let token = jupyterhub_api_token()?;
    let base = jupyterhub_internal_api_url();
    let client = Client::new();
    let auth = format!("token {token}");

    let server_url = format!("{base}/hub/api/users/{username}/server");
    let server_resp = client
        .delete(&server_url)
        .header("Authorization", &auth)
        .send()
        .await
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!(
                "JupyterHub server stop request failed for {username} at {server_url}: {e}"
            ))
        })?;

    let server_status = server_resp.status();
    if server_status.is_success() || server_status.as_u16() == 404 {
        tracing::info!("Stopped JupyterHub server for {username} ({server_status})");
        return Ok(());
    }

    let body = server_resp.text().await.unwrap_or_default();
    tracing::warn!(
        "JupyterHub API stop failed for {username}: {server_status} {body}; falling back to Docker stop"
    );

    
    stop_user_container_via_grading(username).await
}

async fn stop_user_container_via_grading(username: &str) -> Result<(), AppError> {
    let grading_service_url = crate::grading::grading_service_url();
    let client = crate::grading::grading_client()?;
    let stop_url = format!("{grading_service_url}/stop-user/{username}");

    let response = crate::grading::apply_grading_auth(client.post(&stop_url))?
        .send()
        .await
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!(
                "Grading service stop-user request failed for {username}: {e}"
            ))
        })?;

    if response.status().is_success() {
        tracing::info!("Stopped Jupyter container for {username} via grading service");
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(AppError::InternalError(anyhow::anyhow!(
            "Grading service stop-user failed for {username}: {status} {body}"
        )))
    }
}

#[derive(Debug, Deserialize)]
pub struct ValidateTokenRequest {
    pub token: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ValidateTokenResponse {
    pub allowed: bool,
    pub username: Option<String>,
    pub admin: bool,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateSpawnRequest {
    pub username: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ValidateSpawnResponse {
    pub allowed: bool,
    pub message: Option<String>,
}
