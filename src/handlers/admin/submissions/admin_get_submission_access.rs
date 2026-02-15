use axum::{
    Json,
    extract::{Path, State},
};

use crate::{AppState, auth::AdminUser, error::AppError, models::AdminSubmissionAccessResponse};

pub async fn admin_get_submission_access(
    auth: AdminUser,
    State(state): State<AppState>,
    Path(submission_id): Path<uuid::Uuid>,
) -> Result<Json<AdminSubmissionAccessResponse>, AppError> {
    #[derive(sqlx::FromRow)]
    struct SubmissionAccessRow {
        student_jupyterhub_username: Option<String>,
        notebook_filename: String,
    }

    let row: SubmissionAccessRow = sqlx::query_as(
        r#"
        SELECT
            u.jupyterhub_username AS student_jupyterhub_username,
            cn.notebook_filename
        FROM challenge_submissions cs
        JOIN users u ON u.id = cs.user_id
        JOIN challenge_notebooks cn ON cn.id = cs.notebook_id
        WHERE cs.id = $1
        "#,
    )
    .bind(submission_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let student_username = row.student_jupyterhub_username.ok_or_else(|| {
        AppError::BadRequest("Student does not have a JupyterHub username yet".to_string())
    })?;

    let admin_jupyterhub_username = format!("admin_{}", auth.user_id.to_string().replace("-", ""));

    sqlx::query(
        "UPDATE users SET jupyterhub_username = $1 WHERE id = $2 AND jupyterhub_username IS NULL",
    )
    .bind(&admin_jupyterhub_username)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await?;

    let jupyterhub_token =
        crate::auth::create_jupyterhub_token(auth.user_id, &admin_jupyterhub_username)?;

    let jupyterhub_base_url =
        std::env::var("JUPYTERHUB_URL").unwrap_or_else(|_| "http://localhost:8888".to_string());

    let view_next_path = format!(
        "/user/{}/notebooks/{}",
        student_username, row.notebook_filename
    );
    let encoded_view_next = urlencoding::encode(&view_next_path);
    let view_url = format!(
        "{}/hub/login?token={}&next=/hub/spawn/{}?next={}",
        jupyterhub_base_url, jupyterhub_token, student_username, encoded_view_next
    );

    let download_next_path = format!(
        "/user/{}/files/{}?download=1",
        student_username, row.notebook_filename
    );
    let encoded_download_next = urlencoding::encode(&download_next_path);
    let download_url = format!(
        "{}/hub/login?token={}&next=/hub/spawn/{}?next={}",
        jupyterhub_base_url, jupyterhub_token, student_username, encoded_download_next
    );

    Ok(Json(AdminSubmissionAccessResponse {
        success: true,
        view_url,
        download_url,
        message: "Submission access URL generated successfully".to_string(),
    }))
}
