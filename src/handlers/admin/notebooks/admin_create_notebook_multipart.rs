use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{AppState, auth::AdminUser, error::AppError, models::*};

/// Create/upload a notebook for a challenge (admin)
pub async fn admin_create_notebook_multipart(
    _auth: AdminUser,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<AdminItemResponse<AdminChallengeNotebookResponse>>, AppError> {
    use tokio::io::AsyncWriteExt;

    let mut challenge_id: Option<i32> = None;
    let mut assignment_name: Option<String> = None;
    let mut max_points: i32 = 100;
    let mut cpu_limit: f64 = 0.5;
    let mut memory_limit: String = "512M".to_string();
    let mut time_limit_minutes: i32 = 60;
    let mut network_disabled: bool = true;
    let mut notebook_filename: Option<String> = None;
    let mut notebook_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InternalError(e.into()))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "challengeId" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                challenge_id = Some(
                    text.parse()
                        .map_err(|_| AppError::BadRequest("Invalid challengeId".to_string()))?,
                );
            }
            "assignmentName" => {
                assignment_name = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::InternalError(e.into()))?,
                );
            }
            "maxPoints" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                max_points = text.parse().unwrap_or(100);
            }
            "cpuLimit" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                cpu_limit = text.parse().unwrap_or(0.5);
            }
            "memoryLimit" => {
                memory_limit = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
            }
            "timeLimitMinutes" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                time_limit_minutes = text.parse().unwrap_or(60);
            }
            "networkDisabled" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::InternalError(e.into()))?;
                network_disabled = text == "true" || text == "1";
            }
            "notebook" => {
                if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                    notebook_filename = Some(file_name);
                    notebook_data = Some(
                        field
                            .bytes()
                            .await
                            .map_err(|e| AppError::InternalError(e.into()))?
                            .to_vec(),
                    );
                }
            }
            _ => {}
        }
    }

    let challenge_id =
        challenge_id.ok_or_else(|| AppError::BadRequest("Missing challengeId".to_string()))?;
    let assignment_name = assignment_name
        .ok_or_else(|| AppError::BadRequest("Missing assignmentName".to_string()))?;
    let notebook_filename = notebook_filename
        .ok_or_else(|| AppError::BadRequest("Missing notebook file".to_string()))?;
    let notebook_data =
        notebook_data.ok_or_else(|| AppError::BadRequest("Missing notebook file".to_string()))?;

    // Verify challenge exists
    let _challenge: Challenge = sqlx::query_as("SELECT * FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::BadRequest("Challenge not found".to_string()))?;

    // Check if notebook already exists for this challenge
    let existing: Option<ChallengeNotebook> =
        sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
            .bind(challenge_id)
            .fetch_optional(&state.pool)
            .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest(
            "A notebook already exists for this challenge. Delete it first.".to_string(),
        ));
    }

    let existing_assignment: Option<(i32,)> =
        sqlx::query_as("SELECT id FROM challenge_notebooks WHERE assignment_name = $1 LIMIT 1")
            .bind(&assignment_name)
            .fetch_optional(&state.pool)
            .await?;

    if existing_assignment.is_some() {
        return Err(AppError::BadRequest(
            "This assignment name is already in use. Please choose a unique assignment name."
                .to_string(),
        ));
    }

    // Save notebook file
    let notebooks_dir = "uploads/notebooks";
    tokio::fs::create_dir_all(notebooks_dir)
        .await
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to create notebooks directory: {e}"))
        })?;

    let unique_filename = format!("{}_{}", Uuid::new_v4(), notebook_filename);
    let notebook_path = format!("{notebooks_dir}/{unique_filename}");

    let mut file = tokio::fs::File::create(&notebook_path).await.map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to create notebook file: {e}"))
    })?;
    file.write_all(&notebook_data).await.map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to write notebook file: {e}"))
    })?;

    // Insert into database
    let notebook_result = sqlx::query_as(
        r#"
        INSERT INTO challenge_notebooks 
        (challenge_id, assignment_name, notebook_filename, notebook_path, max_points, cpu_limit, memory_limit, time_limit_minutes, network_disabled)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(challenge_id)
    .bind(&assignment_name)
    .bind(&notebook_filename)
    .bind(&notebook_path)
    .bind(max_points)
    .bind(cpu_limit)
    .bind(&memory_limit)
    .bind(time_limit_minutes)
    .bind(network_disabled)
    .fetch_one(&state.pool)
    .await;

    let notebook: ChallengeNotebook = match notebook_result {
        Ok(notebook) => notebook,
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
            let message = match db_err.constraint() {
                Some("challenge_notebooks_assignment_name_key") => {
                    "This assignment name is already in use. Please choose a unique assignment name."
                }
                Some("unique_challenge_notebook") => {
                    "A notebook already exists for this challenge. Delete it first."
                }
                _ => "Notebook already exists for this challenge or assignment.",
            };

            return Err(AppError::BadRequest(message.to_string()));
        }
        Err(e) => return Err(AppError::DatabaseError(e)),
    };

    let response = AdminChallengeNotebookResponse {
        id: notebook.id,
        challenge_id: notebook.challenge_id,
        assignment_name: notebook.assignment_name,
        notebook_filename: notebook.notebook_filename,
        notebook_path: notebook.notebook_path,
        max_points: notebook.max_points,
        cpu_limit: notebook.cpu_limit,
        memory_limit: notebook.memory_limit,
        time_limit_minutes: notebook.time_limit_minutes,
        network_disabled: notebook.network_disabled,
        created_at: notebook.created_at,
        updated_at: notebook.updated_at,
    };

    Ok(Json(AdminItemResponse { item: response }))
}
