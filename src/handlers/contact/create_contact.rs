use axum::{Json, extract::State, http::HeaderMap};

use crate::{AppState, error::AppError, models::*};

const MAX_MESSAGE_CHARS: usize = 5000;

fn client_ip(headers: &HeaderMap) -> Option<String> {
    // Prefer `X-Real-IP`: the reverse proxy (nginx) overwrites this with the real
    // peer address on every request, so clients cannot spoof it. Fall back to the
    // RIGHT-most `X-Forwarded-For` entry, which is the hop appended by our own
    // proxy; the left-most entries are client-supplied and therefore spoofable.
    // Using the left-most token here would let an attacker rotate a fake IP to
    // defeat the per-IP rate limit below.
    headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|ip| !ip.is_empty())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next_back())
                .map(str::trim)
                .filter(|ip| !ip.is_empty())
                .map(str::to_string)
        })
}

async fn count_recent_by_email(
    pool: &sqlx::PgPool,
    email: &str,
    within_hour: bool,
) -> Result<i64, AppError> {
    let count: (i64,) = if within_hour {
        sqlx::query_as(
            "SELECT COUNT(*) FROM contact_messages WHERE LOWER(email) = $1 AND created_at > NOW() - INTERVAL '1 hour'",
        )
        .bind(email)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM contact_messages WHERE LOWER(email) = $1 AND created_at > NOW() - INTERVAL '1 minute'",
        )
        .bind(email)
        .fetch_one(pool)
        .await?
    };
    Ok(count.0)
}

async fn count_recent_by_ip(
    pool: &sqlx::PgPool,
    ip: &str,
    within_hour: bool,
) -> Result<i64, AppError> {
    let count: (i64,) = if within_hour {
        sqlx::query_as(
            "SELECT COUNT(*) FROM contact_messages WHERE sender_ip = $1 AND created_at > NOW() - INTERVAL '1 hour'",
        )
        .bind(ip)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM contact_messages WHERE sender_ip = $1 AND created_at > NOW() - INTERVAL '1 minute'",
        )
        .bind(ip)
        .fetch_one(pool)
        .await?
    };
    Ok(count.0)
}

async fn enforce_rate_limits(
    pool: &sqlx::PgPool,
    email: &str,
    ip: Option<&str>,
) -> Result<(), AppError> {
    if count_recent_by_email(pool, email, false).await? >= 1 {
        return Err(AppError::TooManyRequests(
            "Please wait a minute before sending another message.".to_string(),
        ));
    }

    if count_recent_by_email(pool, email, true).await? >= 10 {
        return Err(AppError::TooManyRequests(
            "You have reached the limit of 10 messages per hour.".to_string(),
        ));
    }

    if let Some(ip) = ip {
        if count_recent_by_ip(pool, ip, false).await? >= 1 {
            return Err(AppError::TooManyRequests(
                "Please wait a minute before sending another message.".to_string(),
            ));
        }

        if count_recent_by_ip(pool, ip, true).await? >= 10 {
            return Err(AppError::TooManyRequests(
                "You have reached the limit of 10 messages per hour.".to_string(),
            ));
        }
    }

    Ok(())
}

pub async fn create_contact(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ContactRequest>,
) -> Result<Json<ContactResponse>, AppError> {
    let name = req.name.trim();
    let email = req.email.trim().to_lowercase();
    let message = req.message.trim();

    if name.is_empty() || email.is_empty() || message.is_empty() {
        return Err(AppError::BadRequest(
            "Name, email, and message are required.".to_string(),
        ));
    }

    if !email.contains('@') || email.len() > 255 || name.len() > 255 {
        return Err(AppError::BadRequest("Invalid contact details.".to_string()));
    }

    if message.chars().count() > MAX_MESSAGE_CHARS {
        return Err(AppError::BadRequest(format!(
            "Message is too long (max {MAX_MESSAGE_CHARS} characters)."
        )));
    }

    let sender_ip = client_ip(&headers);
    enforce_rate_limits(&state.pool, &email, sender_ip.as_deref()).await?;

    sqlx::query(
        "INSERT INTO contact_messages (name, email, message, sender_ip, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(name)
    .bind(&email)
    .bind(message)
    .bind(sender_ip.as_deref())
    .execute(&state.pool)
    .await?;

    Ok(Json(ContactResponse {
        success: true,
        message: "Message sent successfully".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::client_ip;
    use axum::http::HeaderMap;

    #[test]
    fn prefers_trusted_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9, 203.0.113.9".parse().unwrap());
        headers.insert("x-real-ip", "203.0.113.9".parse().unwrap());
        assert_eq!(client_ip(&headers).as_deref(), Some("203.0.113.9"));
    }

    #[test]
    fn ignores_spoofed_leftmost_forwarded_for() {
        // Attacker supplies a fake IP; nginx appends the real peer on the right.
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9, 203.0.113.9".parse().unwrap());
        assert_eq!(client_ip(&headers).as_deref(), Some("203.0.113.9"));
    }

    #[test]
    fn uses_only_forwarded_for_entry_when_single() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());
        assert_eq!(client_ip(&headers).as_deref(), Some("203.0.113.9"));
    }

    #[test]
    fn trims_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.9 ".parse().unwrap());
        assert_eq!(client_ip(&headers).as_deref(), Some("203.0.113.9"));
    }

    #[test]
    fn returns_none_without_ip_headers() {
        let headers = HeaderMap::new();
        assert_eq!(client_ip(&headers), None);
    }
}
