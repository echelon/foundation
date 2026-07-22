//! `outputs` table (files.md §2) — provenance rows tying jobs to their result files.
//! `output_index` == the slice index for fanned-out jobs.

use chrono::{DateTime, Utc};
use ids::{FileId, JobId, OutputId};

mod create_output;
mod list_outputs_with_files_for_job;

pub use create_output::{create_output, NewOutput};
pub use list_outputs_with_files_for_job::{list_outputs_with_files_for_job, OutputWithFile};

#[derive(Debug, Clone)]
pub struct Output {
  pub id: OutputId,
  pub job_id: JobId,
  pub file_id: Option<FileId>,
  pub output_index: i32,
  pub kind: String,
  pub width: Option<i32>,
  pub height: Option<i32>,
  pub created_at: DateTime<Utc>,
}
