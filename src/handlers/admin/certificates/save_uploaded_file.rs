use crate::error::AppError;

pub async fn save_uploaded_file(
    _field_name: &str,
    file_name: &str,
    data: &[u8],
    subdirectory: &str,
) -> Result<String, AppError> {
    use tokio::io::AsyncWriteExt;

    let upload_dir = format!("uploads/{subdirectory}");

    tokio::fs::create_dir_all(&upload_dir).await.map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to create upload directory: {e}"))
    })?;

    let unique_filename = format!("{}_{}", uuid::Uuid::new_v4(), file_name);
    let file_path = format!("{upload_dir}/{unique_filename}");

    let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to create file: {e}"))
    })?;

    file.write_all(data).await.map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to write file: {e}"))
    })?;

    Ok(format!("/{upload_dir}/{unique_filename}"))
}
