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

/// Delete a notebook (admin)
pub async fn admin_delete_notebook(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<i32>,
) -> Result<Json<AdminSuccessResponse>, AppError> {
    // Get notebook to delete the file
    let notebook: Option<ChallengeNotebook> =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE id = $1")
            .bind(notebook_id)
            .fetch_optional(&state.pool)
            .await?;

    if let Some(nb) = notebook {
        // Delete the file
        let _ = tokio::fs::remove_file(&nb.notebook_path).await;
    }

    let result = sqlx::query("DELETE FROM challenge_notebooks WHERE id = $1")
        .bind(notebook_id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(AdminSuccessResponse { success: true }))
}
