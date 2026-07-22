use sqlx::PgExecutor;

use super::Job;

/// UNSLICED job by its provider handle — how a webhook finds the job it's about.
pub async fn find_job_by_provider_job_id<'e, E: PgExecutor<'e>>(
  exec: E,
  provider: &str,
  provider_job_id: &str,
) -> sqlx::Result<Option<Job>> {
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
       FROM jobs WHERE provider = $1 AND provider_job_id = $2
       ORDER BY sid DESC LIMIT 1"#,
    provider,
    provider_job_id,
  )
  .fetch_optional(exec)
  .await
}
