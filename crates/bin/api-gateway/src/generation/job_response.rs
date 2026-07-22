//! The Job resource as APIs report it (api_overview.md §generate + jobs.md §5), including
//! fan-out progress so pollers see PARTIAL completion as slices land.

use serde::Serialize;
use utoipa::ToSchema;

use bucket_paths::{cdn_url, object_key, BucketFolder, ObjectKeyParts};
use db_queries::outputs::OutputWithFile;
use db_queries::{Job, JobSlice};
use ids::JobId;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct JobOutputBody {
  pub index: i32,
  pub kind: String,
  /// The CDN URL (derived — never stored).
  pub url: String,
  pub mime: String,
  pub bytes: i64,
}

/// Fan-out progress — present only for jobs that fanned out into slices. `completed`
/// counts terminal slices; outputs appear incrementally as slices succeed.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobProgressBody {
  pub completed: u32,
  pub total: u32,
  pub succeeded: u32,
  pub failed: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobBody {
  #[schema(value_type = String, example = "job_01k0…")]
  pub id: JobId,
  pub model: String,
  pub modality: String,
  /// Which door created the job: `dashboard` (playground/session) or `api_key`.
  #[schema(value_type = String, example = "dashboard")]
  pub source: enums::JobSource,
  /// Which API plane the request came through: `any` or `unified` (model-specific).
  #[schema(value_type = String, example = "unified")]
  pub api_shape: enums::ApiShape,
  #[schema(value_type = String, example = "running")]
  pub status: enums::JobStatus,
  /// True for dry-run jobs: nothing was sent to the provider; the job is terminal
  /// (`canceled`, failure_code `dry_run`) and the would-be payloads live in the request log.
  pub dry_run: bool,
  /// Present only for fanned-out (n>1) jobs — partial completion surface.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub progress: Option<JobProgressBody>,
  /// Outputs ingested SO FAR (grows while a fanned-out job is still running).
  pub outputs: Vec<JobOutputBody>,
  pub failure_code: Option<String>,
  pub failure_message: Option<String>,
  /// Estimated total price in USD micros (billing is stubbed this iteration).
  pub estimated_micros: i64,
  /// Param adjustments the router made (api_any.md) — pruned/coerced params.
  pub param_adjustments: serde_json::Value,
  /// The sanitized, RESOLVED request as persisted (`StoredImageParams`: file refs already
  /// CDN URLs, n post-clamp) — enough for a client to recreate the request.
  pub request_params: serde_json::Value,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn output_url(state: &AppState, o: &OutputWithFile) -> String {
  let folder = match o.bucket_folder.as_str() {
    "uploads" => BucketFolder::Uploads,
    "samples" => BucketFolder::Samples,
    "avatars" => BucketFolder::Avatars,
    _ => BucketFolder::Generations,
  };
  let key = object_key(ObjectKeyParts {
    folder,
    hash: &o.bucket_hash,
    prefix: None,
    suffix: o.bucket_suffix.as_deref(),
    version: None,
    extension: o.bucket_extension.as_deref(),
  });
  cdn_url(&state.cdn_base_url, &key)
}

pub fn job_body(state: &AppState, job: &Job, slices: &[JobSlice], outputs: &[OutputWithFile]) -> JobBody {
  let progress = (!slices.is_empty()).then(|| {
    let terminal = slices.iter().filter(|s| s.status.is_terminal()).count() as u32;
    JobProgressBody {
      completed: terminal,
      total: slices.len() as u32,
      succeeded: slices.iter().filter(|s| s.status == enums::JobSliceStatus::Succeeded).count() as u32,
      failed: slices.iter().filter(|s| s.status == enums::JobSliceStatus::Failed).count() as u32,
    }
  });
  let estimated = (job.unit_amount_nanos / 1_000) * job.quantity.unwrap_or(1.0).round() as i64;
  JobBody {
    id: job.id.clone(),
    model: job.model.clone(),
    modality: job.modality.clone(),
    source: job.source.clone(),
    api_shape: job.api_shape.clone(),
    status: job.status.clone(),
    dry_run: job.is_dry_run,
    progress,
    outputs: outputs
      .iter()
      .map(|o| JobOutputBody {
        index: o.output_index,
        kind: o.kind.clone(),
        url: output_url(state, o),
        mime: o.mime.clone(),
        bytes: o.bytes,
      })
      .collect(),
    failure_code: job.failure_code.clone(),
    failure_message: job.failure_message.clone(),
    estimated_micros: estimated,
    param_adjustments: job.param_adjustments.clone(),
    request_params: job.request_params.clone(),
    created_at: job.created_at,
    completed_at: job.completed_at,
  }
}
