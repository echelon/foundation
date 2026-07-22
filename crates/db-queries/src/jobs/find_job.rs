use ids::JobId;
use sqlx::PgExecutor;

use super::Job;

/// Cross-account lookup — for the DISPATCHER's poll pass only (it works the whole queue).
/// API handlers must use `find_job_for_account` so tenancy is enforced in the query.
pub async fn find_job<'e, E: PgExecutor<'e>>(exec: E, id: &JobId) -> sqlx::Result<Option<Job>> {
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
       FROM jobs WHERE id = $1"#,
    id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
