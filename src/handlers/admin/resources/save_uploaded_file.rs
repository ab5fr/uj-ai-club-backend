use crate::error::AppError;

pub async fn save_uploaded_file(
    _field_name: &str,
    file_name: &str,
    data: &[u8],
    subdirectory: &str,
) -> Result<String, AppError> {
    use tokio::io::AsyncWriteExt;

    let upload_dir = format!("uploads/{subdirectory}");

    tracing::info!("Creating directory: {}", upload_dir);

    tokio::fs::create_dir_all(&upload_dir).await.map_err(|e| {
        tracing::error!("Failed to create directory {}: {}", upload_dir, e);
        AppError::InternalError(anyhow::anyhow!("Failed to create upload directory: {e}"))
    })?;

    let unique_filename = format!("{}_{}", uuid::Uuid::new_v4(), file_name);
    let file_path = format!("{upload_dir}/{unique_filename}");

    tracing::info!("Saving file to: {}", file_path);

    let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| {
        tracing::error!("Failed to create file {}: {}", file_path, e);
        AppError::InternalError(anyhow::anyhow!("Failed to create file: {e}"))
    })?;

    file.write_all(data).await.map_err(|e| {
        tracing::error!("Failed to write file {}: {}", file_path, e);
        AppError::InternalError(anyhow::anyhow!("Failed to write file: {e}"))
    })?;

    let result_url = format!("/{upload_dir}/{unique_filename}");
    tracing::info!("File saved successfully: {}", result_url);

    Ok(result_url)
}
