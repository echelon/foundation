use sqlx::PgExecutor;

use super::Job;

/// Running single-call jobs (no slices — the jobs row itself carries the provider handle)
/// that need a provider poll.
pub async fn list_pollable_unsliced_jobs<'e, E: PgExecutor<'e>>(
  exec: E,
  provider: &str,
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
                 is_dry_run, created_at, completed_at, expires_at
       FROM jobs
       WHERE status = 'running' AND provider = $2 AND provider_job_id IS NOT NULL
       ORDER BY sid
       LIMIT $1"#,
    limit,
    provider,
  )
  .fetch_all(exec)
  .await
}
