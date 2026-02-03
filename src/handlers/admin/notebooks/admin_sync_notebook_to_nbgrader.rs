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

/// Sync notebook to nbgrader source directory for grading setup
/// This endpoint triggers the grading service to set up the assignment properly
pub async fn admin_sync_notebook_to_nbgrader(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<i32>,
) -> Result<Json<AdminSyncNotebookResponse>, AppError> {
    // Get the notebook
    let notebook: ChallengeNotebook =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE id = $1")
            .bind(notebook_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;

    // Call the grading service to sync the notebook
    let grading_service_url = std::env::var("GRADING_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:9100".to_string());

    let client = reqwest::Client::new();
    let sync_url = format!(
        "{}/setup-assignment/{}",
        grading_service_url, notebook.assignment_name
    );

    // Get the notebook path relative to the uploads directory
    let notebook_path = format!(
        "/srv/notebooks/{}",
        notebook.notebook_path.replace("uploads/", "")
    );

    let payload = serde_json::json!({
        "notebookPath": notebook_path,
        "assignmentName": notebook.assignment_name,
        "maxPoints": notebook.max_points
    });

    tracing::info!(
        "Syncing notebook {} to nbgrader source",
        notebook.assignment_name
    );

    match client.post(&sync_url).json(&payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await.unwrap_or_default();
                tracing::info!("Notebook synced successfully: {:?}", result);
                Ok(Json(AdminSyncNotebookResponse {
                    success: true,
                    message: format!(
                        "Notebook '{}' synced to nbgrader. Students will now receive the graded version.",
                        notebook.assignment_name
                    ),
                }))
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                tracing::warn!("Failed to sync notebook: {} - {}", status, error_text);
                Ok(Json(AdminSyncNotebookResponse {
                    success: false,
                    message: format!("Failed to sync: {} - {}", status, error_text),
                }))
            }
        }
        Err(e) => {
            tracing::warn!("Failed to call grading service: {}", e);
            Ok(Json(AdminSyncNotebookResponse {
                success: false,
                message: format!("Failed to connect to grading service: {}", e),
            }))
        }
    }
}
