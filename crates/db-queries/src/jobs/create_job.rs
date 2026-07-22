use chrono::{DateTime, Utc};
use enums::{ApiShape, JobSource};
use ids::{AccountId, ApiKeyId, JobId, UserId};
use sqlx::PgExecutor;

use super::Job;

#[derive(Debug)]
pub struct NewJob<'a> {
  pub account_id: &'a AccountId,
  pub api_key_id: Option<&'a ApiKeyId>,
  pub user_id: Option<&'a UserId>,
  pub source: JobSource,
  pub media_type: &'a str,
  pub model: &'a str,
  pub modality: &'a str,
  pub api_shape: ApiShape,
  pub api_version: chrono::NaiveDate,
  pub request_params: &'a serde_json::Value,
  pub param_adjustments: &'a serde_json::Value,
  pub provider: &'a str,
  /// Catalog headline unit price (nano-USD).
  pub unit_amount_nanos: i64,
  /// e.g. image count `n`.
  pub quantity: f64,
  pub estimated_max_micros: Option<i64>,
  /// Dry-run request: admitted/priced/logged but never dispatched (born terminal).
  pub is_dry_run: bool,
  pub expires_at: DateTime<Utc>,
}

/// Insert a job born `queued` / `at_our_gateway`, dispatchable immediately. Billing is
/// v0-stubbed: `billing_mode=fixed`, `billing_state=not_charged` (no charging yet).
pub async fn create_job<'e, E: PgExecutor<'e>>(exec: E, new: NewJob<'_>) -> sqlx::Result<Job> {
  let id = JobId::generate();
  sqlx::query_as!(
    Job,
    r#"INSERT INTO jobs
         (id, account_id, api_key_id, user_id, source, media_type, model, modality, api_shape,
          api_version, request_params, param_adjustments, status, queue_stage, dispatch_attempts,
          provider, billing_mode, billing_state, price_unit, unit_amount_nanos, quantity,
          estimated_max_micros, is_dry_run, refunded_micros, next_dispatch_at, expires_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'queued', 'at_our_gateway', 0,
               $13, 'fixed', 'not_charged', 'per_image', $14, $15::float8::numeric, $16, $17, 0, now(), $18)
       RETURNING id AS "id: ids::JobId", account_id AS "account_id: ids::AccountId",
                 api_key_id AS "api_key_id?: ids::ApiKeyId", user_id AS "user_id?: ids::UserId",
                 source AS "source: enums::JobSource", media_type, model, modality,
                 api_shape AS "api_shape: enums::ApiShape", request_params, param_adjustments,
                 status AS "status: enums::JobStatus", queue_stage AS "queue_stage?: enums::QueueStage",
                 dispatch_attempts, failure_code, failure_message, provider, provider_job_id,
                 unit_amount_nanos, quantity::float8 AS "quantity?: f64", estimated_max_micros,
                 is_dry_run, created_at, completed_at, expires_at"#,
    id.as_str(),
    new.account_id.as_str(),
    new.api_key_id.map(|k| k.as_str()),
    new.user_id.map(|u| u.as_str()),
    new.source.as_str(),
    new.media_type,
    new.model,
    new.modality,
    new.api_shape.as_str(),
    new.api_version,
    new.request_params,
    new.param_adjustments,
    new.provider,
    new.unit_amount_nanos,
    new.quantity,
    new.estimated_max_micros,
    new.is_dry_run,
    new.expires_at,
  )
  .fetch_one(exec)
  .await
}
