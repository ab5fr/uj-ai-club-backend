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

pub async fn admin_delete_resource(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdminSuccessResponse>, AppError> {
    let result = sqlx::query("DELETE FROM resources WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(AdminSuccessResponse { success: true }))
}
