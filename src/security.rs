
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

const WEAK_SECRET_PLACEHOLDERS: &[&str] = &["change_me", "default-token", "password", "secret"];
const MIN_SECRET_LEN: usize = 32;

/// Whether strong secrets are required. Defaults to true unless `APP_ENV=dev`
/// (or `REQUIRE_STRONG_SECRETS=false` explicitly).
pub fn require_strong_secrets() -> bool {
    if let Ok(val) = std::env::var("REQUIRE_STRONG_SECRETS") {
        return !matches!(val.to_lowercase().as_str(), "0" | "false" | "no");
    }
    let app_env = std::env::var("APP_ENV")
        .or_else(|_| std::env::var("RUST_ENV"))
        .unwrap_or_else(|_| "production".to_string())
        .to_lowercase();
    !matches!(app_env.as_str(), "dev" | "development" | "local")
}

/// Validate that a named secret is present and strong enough for the current env.
pub fn validate_secret(name: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{name} must be set to a non-empty value"));
    }
    if !require_strong_secrets() {
        return Ok(());
    }
    let lower = trimmed.to_lowercase();
    if WEAK_SECRET_PLACEHOLDERS
        .iter()
        .any(|weak| lower == *weak)
    {
        return Err(format!(
            "{name} looks like a weak/placeholder secret; set a strong value \
             (or set APP_ENV=dev / REQUIRE_STRONG_SECRETS=false for local development)"
        ));
    }
    if trimmed.len() < MIN_SECRET_LEN {
        return Err(format!(
            "{name} must be at least {MIN_SECRET_LEN} characters \
             (or set APP_ENV=dev / REQUIRE_STRONG_SECRETS=false for local development)"
        ));
    }
    Ok(())
}

/// Refuse to start if critical secrets are missing or weak outside explicit dev.
pub fn validate_startup_secrets() -> Result<(), String> {
    for name in ["JWT_SECRET", "GRADING_SERVICE_SECRET", "NBGRADER_WEBHOOK_SECRET"] {
        let value = std::env::var(name).unwrap_or_default();
        validate_secret(name, &value)?;
    }
    // INTERNAL_SERVICE_SECRET is optional: falls back to GRADING_SERVICE_SECRET.
    if let Ok(internal) = std::env::var("INTERNAL_SERVICE_SECRET") {
        if !internal.trim().is_empty() {
            validate_secret("INTERNAL_SERVICE_SECRET", &internal)?;
        }
    }
    Ok(())
}
