use crate::error::AppError;

const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_NOTEBOOK_BYTES: usize = 50 * 1024 * 1024;

pub fn sanitize_filename(file_name: &str) -> String {
    let sanitized: String = std::path::Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("upload")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();

    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        "upload".to_string()
    } else {
        sanitized
    }
}

fn looks_like_image(data: &[u8]) -> bool {
    // JPEG
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return true;
    }
    // PNG
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return true;
    }
    // GIF87a / GIF89a
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return true;
    }
    // WEBP (RIFF....WEBP)
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return true;
    }
    false
}


pub fn assignment_name_from_filename(file_name: &str) -> Result<String, AppError> {
    let sanitized = sanitize_filename(file_name);
    let stem = sanitized
        .strip_suffix(".ipynb")
        .or_else(|| sanitized.strip_suffix(".IPYNB"))
        .unwrap_or(sanitized.as_str())
        .trim_matches(|c| c == '.' || c == '_' || c == '-');

    let assignment_name: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        .collect();

    if assignment_name.is_empty() {
        return Err(AppError::BadRequest(
            "Notebook filename must yield a valid assignment name (letters, numbers, _ or -)"
                .to_string(),
        ));
    }

    Ok(assignment_name)
}

pub fn validate_image_upload(file_name: &str, data: &[u8]) -> Result<(), AppError> {
    if data.is_empty() {
        return Err(AppError::BadRequest("Empty file upload".to_string()));
    }

    if data.len() > MAX_IMAGE_BYTES {
        return Err(AppError::BadRequest(
            "Image file is too large (max 10 MB)".to_string(),
        ));
    }

    let sanitized = sanitize_filename(file_name).to_lowercase();
    let allowed = [".jpg", ".jpeg", ".png", ".webp", ".gif"];
    if !allowed.iter().any(|ext| sanitized.ends_with(ext)) {
        return Err(AppError::BadRequest(
            "Only JPG, PNG, WEBP, and GIF images are allowed".to_string(),
        ));
    }

    if !looks_like_image(data) {
        return Err(AppError::BadRequest(
            "Uploaded file is not a valid JPG, PNG, WEBP, or GIF image".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_notebook_upload(file_name: &str, data: &[u8]) -> Result<(), AppError> {
    if data.is_empty() {
        return Err(AppError::BadRequest("Empty notebook upload".to_string()));
    }

    if data.len() > MAX_NOTEBOOK_BYTES {
        return Err(AppError::BadRequest(
            "Notebook file is too large (max 50 MB)".to_string(),
        ));
    }

    let sanitized = sanitize_filename(file_name).to_lowercase();
    if !sanitized.ends_with(".ipynb") {
        return Err(AppError::BadRequest(
            "Only .ipynb notebook files are allowed".to_string(),
        ));
    }

    Ok(())
}

pub async fn save_uploaded_file(
    _field_name: &str,
    file_name: &str,
    data: &[u8],
    subdirectory: &str,
) -> Result<String, AppError> {
    use tokio::io::AsyncWriteExt;

    if subdirectory.contains("notebooks") {
        validate_notebook_upload(file_name, data)?;
    } else {
        validate_image_upload(file_name, data)?;
    }

    let safe_name = sanitize_filename(file_name);
    let upload_dir = format!("uploads/{subdirectory}");

    tokio::fs::create_dir_all(&upload_dir).await.map_err(|e| {
        tracing::error!("Failed to create directory {}: {}", upload_dir, e);
        AppError::InternalError(anyhow::anyhow!("Failed to create upload directory: {e}"))
    })?;

    let unique_filename = format!("{}_{}", uuid::Uuid::new_v4(), safe_name);
    let file_path = format!("{upload_dir}/{unique_filename}");

    let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| {
        tracing::error!("Failed to create file {}: {}", file_path, e);
        AppError::InternalError(anyhow::anyhow!("Failed to create file: {e}"))
    })?;

    file.write_all(data).await.map_err(|e| {
        tracing::error!("Failed to write file {}: {}", file_path, e);
        AppError::InternalError(anyhow::anyhow!("Failed to write file: {e}"))
    })?;

    Ok(format!("/{upload_dir}/{unique_filename}"))
}
