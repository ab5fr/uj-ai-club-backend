use axum::{Json, extract::State, http::HeaderMap};

use crate::{
    AppState,
    auth,
    error::AppError,
    jupyterhub::{ValidateTokenRequest, ValidateTokenResponse},
    security,
};

pub async fn validate_jupyterhub_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ValidateTokenRequest>,
) -> Result<Json<ValidateTokenResponse>, AppError> {
    let expected_secret = crate::grading::internal_service_secret()?;

    let provided = headers
        .get("X-Grading-Service-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !security::constant_time_eq(provided, &expected_secret) {
        return Err(AppError::AuthError);
    }

    let claims = match auth::decode_jupyterhub_token(&body.token) {
        Ok(c) => c,
        Err(_) => {
            return Ok(Json(ValidateTokenResponse {
                allowed: false,
                username: None,
                admin: false,
                message: Some("Invalid token".to_string()),
            }));
        }
    };

    if claims.purpose == "jupyterhub_admin" {
        let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| AppError::AuthError)?;
        let role: Option<(String,)> =
            sqlx::query_as("SELECT role FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&state.pool)
                .await?;

        let is_admin = role.map(|r| r.0 == "admin").unwrap_or(false);
        return Ok(Json(ValidateTokenResponse {
            allowed: is_admin,
            username: Some(claims.username),
            admin: is_admin,
            message: if is_admin {
                None
            } else {
                Some("Admin access required".to_string())
            },
        }));
    }

    if claims.purpose != "jupyterhub_sso" {
        return Ok(Json(ValidateTokenResponse {
            allowed: false,
            username: None,
            admin: false,
            message: Some("Invalid token purpose".to_string()),
        }));
    }

    let jti = claims.jti.as_deref().unwrap_or("");
    let submission_id = claims
        .submission_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    if jti.is_empty() || submission_id.is_none() {
        return Ok(Json(ValidateTokenResponse {
            allowed: false,
            username: None,
            admin: false,
            message: Some("Missing session claims".to_string()),
        }));
    }

    let submission_id = submission_id.unwrap();
    let now = time::OffsetDateTime::now_utc();

    #[derive(sqlx::FromRow)]
    struct SessionRow {
        session_jti: Option<String>,
        session_revoked_at: Option<time::OffsetDateTime>,
        session_expires_at: Option<time::OffsetDateTime>,
        status: String,
    }

    let row: Option<SessionRow> = sqlx::query_as(
        r#"
        SELECT session_jti, session_revoked_at, session_expires_at, status
        FROM challenge_submissions
        WHERE id = $1
        "#,
    )
    .bind(submission_id)
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        return Ok(Json(ValidateTokenResponse {
            allowed: false,
            username: None,
            admin: false,
            message: Some("Unknown submission".to_string()),
        }));
    };

    if row.status != "in_progress" {
        return Ok(Json(ValidateTokenResponse {
            allowed: false,
            username: Some(claims.username),
            admin: false,
            message: Some("Session no longer active".to_string()),
        }));
    }

    if row.session_revoked_at.is_some() {
        return Ok(Json(ValidateTokenResponse {
            allowed: false,
            username: Some(claims.username),
            admin: false,
            message: Some("Session revoked".to_string()),
        }));
    }

    if row.session_jti.as_deref() != Some(jti) {
        return Ok(Json(ValidateTokenResponse {
            allowed: false,
            username: Some(claims.username),
            admin: false,
            message: Some("Token superseded".to_string()),
        }));
    }

    if let Some(expires) = row.session_expires_at
        && now >= expires
    {
        return Ok(Json(ValidateTokenResponse {
            allowed: false,
            username: Some(claims.username),
            admin: false,
            message: Some("Session expired".to_string()),
        }));
    }

    Ok(Json(ValidateTokenResponse {
        allowed: true,
        username: Some(claims.username),
        admin: false,
        message: None,
    }))
}
