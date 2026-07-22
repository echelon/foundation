use sqlx::PgExecutor;

use super::Job;

/// ALL accounts' recent jobs, newest first — the staff/admin view (keyset via `after`).
pub async fn list_recent_jobs<'e, E: PgExecutor<'e>>(
  exec: E,
  after: Option<&ids::JobId>,
  limit: i64,
) -> sqlx::Result<Vec<Job>> {
  sqlx::query_as!(
    Job,
    r#"SELECT id AS "id: ids::JobId", account_id AS "account_id: ids::AccountId",
                 api_key_id AS "api_key_id?: ids::ApiKeyId", user_id AS "user_id?: ids::UserId",
                 source AS "source: enums::JobSource", media_type, model, modality,
                 api_shape AS "api_shape: enums::ApiShape", request_params, param_adjustments,
                 status AS "status: enums::JobStatus", queue_stage AS "queue_stage?: enums::QueueStage",
                 dispatch_attempts, failure_code, failure_message, provider, provider_job_id,
                 unit_amount_nanos, quantity::float8 AS "quantity?: f64", estimated_max_micros,
                 created_at, completed_at, expires_at
       FROM jobs
       WHERE ($1::text IS NULL OR sid < (SELECT sid FROM jobs WHERE id = $1))
       ORDER BY sid DESC
       LIMIT $2"#,
    after.map(|j| j.as_str()),
    limit,
  )
  .fetch_all(exec)
  .await
}
