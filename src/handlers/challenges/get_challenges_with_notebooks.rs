use axum::{Json, extract::State};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppError,
    models::*,
};

/// Get all challenges with notebook information for the user
pub async fn get_challenges_with_notebooks(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<ChallengeWithNotebookResponse>>, AppError> {
    // Get all visible challenges with their notebook info
    let challenges: Vec<Challenge> = sqlx::query_as(
        r#"
        SELECT * FROM challenges 
        WHERE visible = true 
        ORDER BY week DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let mut responses = Vec::new();

    for challenge in challenges {
        // Check if this challenge has a notebook
        let notebook: Option<ChallengeNotebook> =
            sqlx::query_as("SELECT * FROM challenge_notebooks WHERE challenge_id = $1")
                .bind(challenge.id)
                .fetch_optional(&state.pool)
                .await?;

        responses.push(ChallengeWithNotebookResponse {
            id: challenge.id,
            week: challenge.week,
            title: challenge.title,
            description: challenge.description,
            has_notebook: notebook.is_some(),
            max_points: notebook.as_ref().map(|n| n.max_points),
            time_limit_minutes: notebook.as_ref().map(|n| n.time_limit_minutes),
            start_date: challenge.start_date,
            end_date: challenge.end_date,
        });
    }

    Ok(Json(responses))
}
