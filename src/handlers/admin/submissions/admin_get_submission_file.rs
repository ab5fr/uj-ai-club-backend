use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
};
use serde::Deserialize;

use crate::{AppState, auth::AdminUser, error::AppError};

#[derive(Debug, Deserialize)]
pub struct SubmissionFileQuery {
    pub download: Option<bool>,
}

pub async fn admin_get_submission_file(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(submission_id): Path<uuid::Uuid>,
    Query(query): Query<SubmissionFileQuery>,
) -> Result<Response, AppError> {
    #[derive(sqlx::FromRow)]
    struct SubmissionFileRow {
        student_jupyterhub_username: Option<String>,
        assignment_name: String,
        notebook_filename: String,
    }

    let row: SubmissionFileRow = sqlx::query_as(
        r#"
        SELECT
            u.jupyterhub_username AS student_jupyterhub_username,
            cn.assignment_name,
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

    let grading_service_url =
        std::env::var("GRADING_SERVICE_URL").unwrap_or_else(|_| "http://uj-ai-club-grading:9100".to_string());

    let download = query.download.unwrap_or(false);
    let endpoint = format!(
        "{}/submissions/{}/{}/notebook?download={}",
        grading_service_url,
        urlencoding::encode(&student_username),
        urlencoding::encode(&row.assignment_name),
        if download { 1 } else { 0 }
    );

    let response = reqwest::Client::new()
        .get(&endpoint)
        .send()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::BadRequest(
            "Submitted notebook file was not found yet. Ask the student to submit again, then retry."
                .to_string(),
        ));
    }

    if !response.status().is_success() {
        return Err(AppError::InternalError(anyhow::anyhow!(
            "Failed to fetch submitted notebook from grading service: {}",
            response.status()
        )));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/x-ipynb+json")
        .to_string();

    let content_disposition = if download {
        format!("attachment; filename=\"{}\"", row.notebook_filename)
    } else {
        format!("inline; filename=\"{}\"", row.notebook_filename)
    };

    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let body = Body::from(bytes);
    let response = Response::builder()
        .header(axum::http::header::CONTENT_TYPE, content_type)
        .header(axum::http::header::CONTENT_DISPOSITION, content_disposition)
        .body(body)
        .map_err(|e| AppError::InternalError(e.into()))?;

    Ok(response)
}