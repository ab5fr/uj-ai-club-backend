use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    auth::{self, AdminUser},
    error::AppError,
    jupyterhub,
    models::AdminSubmissionAccessResponse,
};

pub async fn admin_get_submission_access(
    auth: AdminUser,
    State(state): State<AppState>,
    Path(submission_id): Path<uuid::Uuid>,
) -> Result<Json<AdminSubmissionAccessResponse>, AppError> {
    #[derive(sqlx::FromRow)]
    struct SubmissionAccessRow {
        user_id: uuid::Uuid,
        notebook_filename: String,
    }

    let row: SubmissionAccessRow = sqlx::query_as(
        r#"
        SELECT cs.user_id, cn.notebook_filename
        FROM challenge_submissions cs
        JOIN challenge_notebooks cn ON cn.id = cs.notebook_id
        WHERE cs.id = $1
        "#,
    )
    .bind(submission_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let student_username = jupyterhub::student_jupyterhub_username(row.user_id);
    let admin_username = jupyterhub::admin_jupyterhub_username(auth.user_id);
    let jupyterhub_token = auth::create_jupyterhub_admin_token(auth.user_id, &admin_username)?;

    let jupyterhub_base_url = jupyterhub::jupyterhub_public_url();

    let view_next_path = format!(
        "/user/{student_username}/notebooks/{}",
        row.notebook_filename
    );
    let encoded_view_next = urlencoding::encode(&view_next_path);
    let view_url = format!(
        "{jupyterhub_base_url}/hub/login?token={jupyterhub_token}&next=/hub/spawn/{student_username}?next={encoded_view_next}"
    );

    let download_next_path = format!(
        "/user/{student_username}/files/{}?download=1",
        row.notebook_filename
    );
    let encoded_download_next = urlencoding::encode(&download_next_path);
    let download_url = format!(
        "{jupyterhub_base_url}/hub/login?token={jupyterhub_token}&next=/hub/spawn/{student_username}?next={encoded_download_next}"
    );

    Ok(Json(AdminSubmissionAccessResponse {
        success: true,
        view_url,
        download_url,
        message: "Submission access URL generated successfully".to_string(),
    }))
}
