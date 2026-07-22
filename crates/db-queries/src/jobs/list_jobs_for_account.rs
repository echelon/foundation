use ids::AccountId;
use sqlx::PgExecutor;

use super::Job;

/// Optional list filters (jobs.md §5). Keyset pagination: pass the last row's id as `after`.
#[derive(Debug, Default)]
pub struct JobListFilter<'a> {
  pub model: Option<&'a str>,
  /// Keyset cursor: return rows OLDER than this job id.
  pub after: Option<&'a ids::JobId>,
}

/// The account's jobs, newest first.
pub async fn list_jobs_for_account<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  filter: JobListFilter<'_>,
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
       WHERE account_id = $1
         AND ($2::text IS NULL OR model = $2)
         AND ($3::text IS NULL OR sid < (SELECT sid FROM jobs WHERE id = $3))
       ORDER BY sid DESC
       LIMIT $4"#,
    account_id.as_str(),
    filter.model,
    filter.after.map(|j| j.as_str()),
    limit,
  )
  .fetch_all(exec)
  .await
}
