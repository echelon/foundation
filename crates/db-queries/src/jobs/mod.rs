//! `jobs` table (jobs.md) — the universal generation primitive.

use chrono::{DateTime, Utc};
use enums::{ApiShape, JobSource, JobStatus, QueueStage};
use ids::{AccountId, ApiKeyId, JobId, UserId};

mod backoff_job_dispatch;
mod claim_due_dispatch_jobs;
mod complete_job;
mod create_job;
mod find_job;
mod find_job_by_provider_job_id;
mod find_job_for_account;
mod list_jobs_for_account;
mod list_pollable_unsliced_jobs;
mod list_recent_jobs;
mod set_job_provider_accepted;

pub use backoff_job_dispatch::backoff_job_dispatch;
pub use claim_due_dispatch_jobs::claim_due_dispatch_jobs;
pub use complete_job::{complete_job, CompleteJob};
pub use create_job::{create_job, NewJob};
pub use find_job::find_job;
pub use find_job_by_provider_job_id::find_job_by_provider_job_id;
pub use find_job_for_account::find_job_for_account;
pub use list_jobs_for_account::{list_jobs_for_account, JobListFilter};
pub use list_pollable_unsliced_jobs::list_pollable_unsliced_jobs;
pub use list_recent_jobs::list_recent_jobs;
pub use set_job_provider_accepted::set_job_provider_accepted;

/// A row from `jobs` — the columns this iteration reads (billing columns included so
/// responses can show pricing; never `sid`). Field order MUST match every query's
/// SELECT/RETURNING list.
#[derive(Debug, Clone)]
pub struct Job {
  pub id: JobId,
  pub account_id: AccountId,
  pub api_key_id: Option<ApiKeyId>,
  pub user_id: Option<UserId>,
  pub source: JobSource,
  pub media_type: String,
  pub model: String,
  pub modality: String,
  pub api_shape: ApiShape,
  pub request_params: serde_json::Value,
  pub param_adjustments: serde_json::Value,
  pub status: JobStatus,
  pub queue_stage: Option<QueueStage>,
  pub dispatch_attempts: i32,
  pub failure_code: Option<String>,
  pub failure_message: Option<String>,
  pub provider: Option<String>,
  pub provider_job_id: Option<String>,
  pub unit_amount_nanos: i64,
  pub quantity: Option<f64>,
  pub estimated_max_micros: Option<i64>,
  pub is_dry_run: bool,
  pub created_at: DateTime<Utc>,
  pub completed_at: Option<DateTime<Utc>>,
  pub expires_at: DateTime<Utc>,
}
