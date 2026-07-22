//! `job_slices` table (jobs.md addendum 2026-07-22): fan-out units — one row per provider
//! call for jobs whose single API request maps to several (e.g. GPT Image n>1). Rows exist
//! ONLY for fanned-out jobs; `slice_index` maps 1:1 to `outputs.output_index`.

use chrono::{DateTime, Utc};
use enums::JobSliceStatus;
use ids::{JobId, JobSliceId};

mod complete_job_slice;
mod create_job_slices;
mod list_pollable_slices;
mod list_slices_for_job;
mod set_slice_provider_accepted;

pub use complete_job_slice::{complete_job_slice, CompleteJobSlice};
pub use create_job_slices::create_job_slices;
pub use list_pollable_slices::list_pollable_slices;
pub use list_slices_for_job::list_slices_for_job;
pub use set_slice_provider_accepted::set_slice_provider_accepted;

#[derive(Debug, Clone)]
pub struct JobSlice {
  pub id: JobSliceId,
  pub job_id: JobId,
  pub slice_index: i32,
  pub status: JobSliceStatus,
  pub provider_job_id: Option<String>,
  pub failure_code: Option<String>,
  pub failure_message: Option<String>,
  pub created_at: DateTime<Utc>,
  pub dispatched_at: Option<DateTime<Utc>>,
  pub completed_at: Option<DateTime<Utc>>,
}
