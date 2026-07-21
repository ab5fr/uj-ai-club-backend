use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError, handlers::webhooks::update_user_ranks::update_user_ranks,
    models::ChallengeSubmission,
};

pub async fn effective_allowed_submissions(
    pool: &PgPool,
    user_id: Uuid,
    challenge_id: i32,
    base_allowed: i32,
) -> Result<i32, AppError> {
    let extra: Option<i32> = sqlx::query_scalar(
        "SELECT extra_attempts FROM challenge_attempt_overrides WHERE user_id = $1 AND challenge_id = $2",
    )
    .bind(user_id)
    .bind(challenge_id)
    .fetch_optional(pool)
    .await?;

    Ok(base_allowed.max(1) + extra.unwrap_or(0))
}

pub struct ApplyGradeParams {
    pub submission_id: Uuid,
    pub score: f64,
    pub max_score: f64,
    pub manual_graded_by: Option<Uuid>,
}


fn percentage_score(score: f64, max_score: f64, is_manual: bool) -> f64 {
    if is_manual {
        score.clamp(0.0, 100.0)
    } else if max_score > 0.0 {
        ((score / max_score) * 100.0).clamp(0.0, 100.0)
    } else {
        score.clamp(0.0, 100.0)
    }
}


pub async fn sync_best_attempt_points(
    pool: &PgPool,
    user_id: Uuid,
    challenge_id: i32,
) -> Result<(), AppError> {
    #[derive(sqlx::FromRow)]
    struct AttemptRow {
        id: Uuid,
    }

    let attempts: Vec<AttemptRow> = sqlx::query_as(
        r#"
        SELECT id
        FROM challenge_submissions
        WHERE user_id = $1 AND challenge_id = $2 AND status = 'graded'
        ORDER BY points_awarded DESC, graded_at DESC NULLS LAST, attempt_number DESC, created_at DESC
        "#,
    )
    .bind(user_id)
    .bind(challenge_id)
    .fetch_all(pool)
    .await?;

    let best_id = attempts.first().map(|a| a.id);

    sqlx::query(
        r#"
        UPDATE challenge_submissions
        SET points_credited = false, updated_at = NOW()
        WHERE user_id = $1 AND challenge_id = $2 AND points_credited = true
        "#,
    )
    .bind(user_id)
    .bind(challenge_id)
    .execute(pool)
    .await?;

    if let Some(best_id) = best_id {
        sqlx::query(
            r#"
            UPDATE challenge_submissions
            SET points_credited = true, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(best_id)
        .execute(pool)
        .await?;
    }

    
    
    let total_points: i32 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(challenge_best), 0)::int
        FROM (
            SELECT COALESCE(MAX(points_awarded), 0) AS challenge_best
            FROM challenge_submissions
            WHERE user_id = $1 AND status = 'graded'
            GROUP BY challenge_id
        ) AS per_challenge
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    sqlx::query("UPDATE users SET points = $1 WHERE id = $2")
        .bind(total_points)
        .bind(user_id)
        .execute(pool)
        .await?;

    update_user_ranks(pool).await?;
    Ok(())
}


pub async fn apply_grade(
    pool: &PgPool,
    params: ApplyGradeParams,
) -> Result<ChallengeSubmission, AppError> {
    let is_manual = params.manual_graded_by.is_some();

    #[derive(sqlx::FromRow)]
    struct GradeTarget {
        user_id: Uuid,
        challenge_id: i32,
        max_points: i32,
    }

    let target: GradeTarget = sqlx::query_as(
        r#"
        SELECT cs.user_id, cs.challenge_id, cn.max_points
        FROM challenge_submissions cs
        JOIN challenge_notebooks cn ON cn.id = cs.notebook_id
        WHERE cs.id = $1
        "#,
    )
    .bind(params.submission_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    
    let percentage = percentage_score(params.score, params.max_score, is_manual);
    let points_awarded = ((percentage / 100.0) * target.max_points as f64).round() as i32;

    if let Some(admin_id) = params.manual_graded_by {
        sqlx::query_as::<_, ChallengeSubmission>(
            r#"
            UPDATE challenge_submissions
            SET status = 'graded',
                score = $1,
                max_score = 100.0,
                points_awarded = $2,
                graded_at = NOW(),
                manual_graded_by = $3,
                manual_graded_at = NOW(),
                updated_at = NOW()
            WHERE id = $4
            RETURNING *
            "#,
        )
        .bind(percentage)
        .bind(points_awarded)
        .bind(admin_id)
        .bind(params.submission_id)
        .fetch_one(pool)
        .await?;
    } else {
        sqlx::query_as::<_, ChallengeSubmission>(
            r#"
            UPDATE challenge_submissions
            SET status = 'graded',
                score = $1,
                max_score = 100.0,
                points_awarded = $2,
                graded_at = NOW(),
                updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(percentage)
        .bind(points_awarded)
        .bind(params.submission_id)
        .fetch_one(pool)
        .await?;
    }

    sync_best_attempt_points(pool, target.user_id, target.challenge_id).await?;

    
    let refreshed: ChallengeSubmission =
        sqlx::query_as("SELECT * FROM challenge_submissions WHERE id = $1")
            .bind(params.submission_id)
            .fetch_one(pool)
            .await?;

    Ok(refreshed)
}

pub struct FinalizeSubmissionParams<'a> {
    pub submission_id: Uuid,
    pub user_id: Uuid,
    pub notebook_filename: &'a str,
    pub notebook_path: &'a str,
    pub assignment_name: &'a str,
    pub auto_grade_enabled: bool,
}


pub async fn finalize_in_progress_submission(
    pool: &PgPool,
    params: FinalizeSubmissionParams<'_>,
) -> Result<Option<String>, AppError> {
    let jupyterhub_username = crate::jupyterhub::student_jupyterhub_username(params.user_id);

    if let Err(e) =
        crate::grading::save_user_notebook(&jupyterhub_username, params.notebook_filename).await
    {
        tracing::warn!("Pre-submit notebook save failed for {jupyterhub_username}: {e}");
    }

    let updated = sqlx::query(
        r#"
        UPDATE challenge_submissions
        SET status = 'grading_pending',
            submitted_at = NOW(),
            session_revoked_at = NOW(),
            session_jti = NULL,
            updated_at = NOW()
        WHERE id = $1
          AND status = 'in_progress'
          AND session_revoked_at IS NULL
        "#,
    )
    .bind(params.submission_id)
    .execute(pool)
    .await?
    .rows_affected();

    if updated == 0 {
        return Ok(None);
    }

    let grading_service_url = crate::grading::grading_service_url();
    let client = crate::grading::grading_client()?;
    let trigger_url = format!(
        "{}/submit/{}/{}",
        grading_service_url, jupyterhub_username, params.assignment_name
    );

    let payload = serde_json::json!({
        "notebookFilename": params.notebook_filename,
        "notebookPath": params.notebook_path
    });

    match crate::grading::apply_grading_auth(client.post(&trigger_url).json(&payload))?
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            tracing::info!(
                "Grading triggered for user {jupyterhub_username} on assignment {}",
                params.assignment_name
            );
        }
        Ok(response) => {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Failed to trigger grading: {status} - {error_text}");
        }
        Err(e) => {
            tracing::warn!("Failed to call grading service: {e}");
        }
    }

    if let Err(e) =
        crate::grading::close_jupyter_session(&jupyterhub_username, params.notebook_filename).await
    {
        tracing::error!("Failed to close Jupyter session on submit for {jupyterhub_username}: {e}");
    }

    let message = if params.auto_grade_enabled {
        "Submission received. Auto-grading is in progress.".to_string()
    } else {
        "Submission received and marked as grading pending. An admin will review it.".to_string()
    };

    Ok(Some(message))
}
