use axum::{Json, extract::State, http::HeaderMap};
use time::OffsetDateTime;

use crate::{
    AppState,
    error::AppError,
    jupyterhub::{ValidateSpawnRequest, ValidateSpawnResponse},
    security,
};


pub async fn validate_jupyterhub_spawn(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ValidateSpawnRequest>,
) -> Result<Json<ValidateSpawnResponse>, AppError> {
    let expected_secret = crate::grading::internal_service_secret()?;

    let provided = headers
        .get("X-Grading-Service-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !security::constant_time_eq(provided, &expected_secret) {
        return Err(AppError::AuthError);
    }

    let username = body.username.trim();
    if username.is_empty() {
        return Ok(Json(ValidateSpawnResponse {
            allowed: false,
            message: Some("Missing username".to_string()),
        }));
    }

    if let Some(admin_id_hex) = username.strip_prefix("admin_") {
        let is_admin: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM users
                WHERE role = 'admin'
                  AND REPLACE(id::text, '-', '') = $1
            )
            "#,
        )
        .bind(admin_id_hex)
        .fetch_one(&state.pool)
        .await?;

        return Ok(Json(ValidateSpawnResponse {
            allowed: is_admin,
            message: if is_admin {
                None
            } else {
                Some("Admin access required".to_string())
            },
        }));
    }

    if !username.starts_with("user_") {
        return Ok(Json(ValidateSpawnResponse {
            allowed: false,
            message: Some("Unknown JupyterHub username".to_string()),
        }));
    }

    let now = OffsetDateTime::now_utc();

    let allowed: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM challenge_submissions cs
            INNER JOIN users u ON u.id = cs.user_id
            WHERE u.jupyterhub_username = $1
              AND cs.status = 'in_progress'
              AND cs.session_revoked_at IS NULL
              AND cs.session_jti IS NOT NULL
              AND cs.session_expires_at IS NOT NULL
              AND cs.session_expires_at > $2
        )
        "#,
    )
    .bind(username)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(ValidateSpawnResponse {
        allowed,
        message: if allowed {
            None
        } else {
            Some(
                "Challenge time is up or no active notebook session. Return to the challenges page to submit or start a new attempt."
                    .to_string(),
            )
        },
    }))
}
