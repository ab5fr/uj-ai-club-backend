use time::OffsetDateTime;
use uuid::Uuid;

use crate::submissions::{FinalizeSubmissionParams, finalize_in_progress_submission};

#[derive(sqlx::FromRow)]
struct ExpiredSessionRow {
    id: Uuid,
    user_id: Uuid,
    notebook_filename: String,
    notebook_path: String,
    assignment_name: String,
    auto_grade_enabled: bool,
}

/// Background worker that auto-submits attempts whose time limit has expired.
pub async fn run_session_expiry_worker(pool: sqlx::PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        if let Err(e) = auto_submit_expired_sessions(&pool).await {
            tracing::warn!("Session expiry worker error: {e}");
        }
    }
}

async fn auto_submit_expired_sessions(pool: &sqlx::PgPool) -> Result<(), anyhow::Error> {
    let now = OffsetDateTime::now_utc();

    let rows: Vec<ExpiredSessionRow> = sqlx::query_as(
        r#"
        SELECT
            cs.id,
            cs.user_id,
            cn.notebook_filename,
            cn.notebook_path,
            cn.assignment_name,
            cn.auto_grade_enabled
        FROM challenge_submissions cs
        INNER JOIN challenge_notebooks cn ON cn.challenge_id = cs.challenge_id
        WHERE cs.status = 'in_progress'
          AND cs.session_revoked_at IS NULL
          AND cs.session_expires_at IS NOT NULL
          AND cs.session_expires_at <= $1
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    for row in rows {
        tracing::info!(
            "Auto-submitting expired challenge attempt {} for user {}",
            row.id,
            row.user_id
        );

        match finalize_in_progress_submission(
            pool,
            FinalizeSubmissionParams {
                submission_id: row.id,
                user_id: row.user_id,
                notebook_filename: &row.notebook_filename,
                notebook_path: &row.notebook_path,
                assignment_name: &row.assignment_name,
                auto_grade_enabled: row.auto_grade_enabled,
            },
        )
        .await
        {
            Ok(Some(_)) => {
                tracing::info!("Auto-submitted expired attempt {}", row.id);
            }
            Ok(None) => {
                tracing::info!("Expired attempt {} already finalized", row.id);
            }
            Err(e) => {
                tracing::warn!("Failed to auto-submit expired attempt {}: {e}", row.id);
            }
        }
    }

    Ok(())
}
