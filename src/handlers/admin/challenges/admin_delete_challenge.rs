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

pub async fn admin_delete_challenge(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdminSuccessResponse>, AppError> {
    // Before deleting the challenge, clean up any associated notebook file and nbgrader assignment
    let notebook: Option<ChallengeNotebook> =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;

    if let Some(nb) = &notebook {
        // Delete the notebook file from disk
        let _ = tokio::fs::remove_file(&nb.notebook_path).await;

        // Clean up the nbgrader assignment in the grading service (best-effort)
        let assignment_name = nb.assignment_name.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::grading::cleanup_nbgrader_assignment(&assignment_name).await {
                tracing::warn!("nbgrader cleanup after challenge delete failed: {e}");
            }
        });
    }

    // Remove this challenge's credited points from users before cascade-delete.
    #[derive(sqlx::FromRow)]
    struct CreditedRow {
        user_id: uuid::Uuid,
        points_awarded: i32,
    }

    let credited: Vec<CreditedRow> = sqlx::query_as(
        r#"
        SELECT user_id, points_awarded
        FROM challenge_submissions
        WHERE challenge_id = $1 AND points_credited = true
        "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    for row in &credited {
        sqlx::query("UPDATE users SET points = GREATEST(0, points - $1) WHERE id = $2")
            .bind(row.points_awarded)
            .bind(row.user_id)
            .execute(&state.pool)
            .await?;
    }

    // Delete the challenge row (cascade deletes challenge_notebooks and challenge_submissions)
    let result = sqlx::query("DELETE FROM challenges WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    if !credited.is_empty() {
        crate::handlers::webhooks::update_user_ranks::update_user_ranks(&state.pool).await?;
    }

    Ok(Json(AdminSuccessResponse { success: true }))
}
