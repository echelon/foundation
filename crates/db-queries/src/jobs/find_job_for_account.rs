use ids::{AccountId, JobId};
use sqlx::PgExecutor;

use super::Job;

/// Account-scoped fetch — foreign job ids are indistinguishable from missing ones.
pub async fn find_job_for_account<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  id: &JobId,
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
                 created_at, completed_at, expires_at
       FROM jobs WHERE id = $1 AND account_id = $2"#,
    id.as_str(),
    account_id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
