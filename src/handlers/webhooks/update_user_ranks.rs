use crate::error::AppError;

pub async fn update_user_ranks(pool: &sqlx::PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE users SET rank = ranked.new_rank
        FROM (
            SELECT id, ROW_NUMBER() OVER (ORDER BY points DESC) as new_rank
            FROM users
        ) AS ranked
        WHERE users.id = ranked.id
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
