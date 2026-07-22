use sqlx::PgExecutor;

use super::Job;

/// Claim up to `limit` dispatch-due queued jobs by LEASING them: `next_dispatch_at` jumps
/// `lease_seconds` into the future in the same statement (SKIP LOCKED under concurrent
/// claimers), so a crashed dispatcher's claims simply come due again. Status stays
/// `queued` until the provider accepts (jobs.md: retries are invisible).
pub async fn claim_due_dispatch_jobs<'e, E: PgExecutor<'e>>(
  exec: E,
  lease_seconds: f64,
  limit: i64,
) -> sqlx::Result<Vec<Job>> {
  sqlx::query_as!(
    Job,
    r#"UPDATE jobs SET next_dispatch_at = now() + make_interval(secs => $1),
                       dispatch_attempts = dispatch_attempts + 1
       WHERE id IN (
         SELECT id FROM jobs
         WHERE status = 'queued' AND next_dispatch_at <= now() AND expires_at > now()
         ORDER BY sid
         LIMIT $2
         FOR UPDATE SKIP LOCKED
       )
       RETURNING id AS "id: ids::JobId", account_id AS "account_id: ids::AccountId",
                 api_key_id AS "api_key_id?: ids::ApiKeyId", user_id AS "user_id?: ids::UserId",
                 source AS "source: enums::JobSource", media_type, model, modality,
                 api_shape AS "api_shape: enums::ApiShape", request_params, param_adjustments,
                 status AS "status: enums::JobStatus", queue_stage AS "queue_stage?: enums::QueueStage",
                 dispatch_attempts, failure_code, failure_message, provider, provider_job_id,
                 unit_amount_nanos, quantity::float8 AS "quantity?: f64", estimated_max_micros,
                 created_at, completed_at, expires_at"#,
    lease_seconds,
    limit,
  )
  .fetch_all(exec)
  .await
}
