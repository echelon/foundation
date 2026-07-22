//! `GET /v1/jobs/{id}/request` — the persisted request behind a job, TYPED, in the same
//! wire shape the generate endpoints accept: POST the fields straight back to
//! `/v1/models/{model}/generate` to replay it (image refs come back as CDN URLs, which
//! `image_inputs` accepts). This keeps the Job resource itself free of raw param blobs.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use ids::JobId;

use super::get_job_handler::JobScopeParams;
use super::types::{
  resolve_caller_account, ApiImageQuality, ApiImageSize, ApiInputFidelity, ApiOutputFormat,
};
use crate::error::{ApiError, ApiResult};
use crate::generation::stored_params::StoredImageParams;
use crate::middleware::Caller;
use crate::state::AppState;

/// A job's originating request, replayable against the generate endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobRequestResponse {
  #[schema(value_type = String, example = "job_01k0…")]
  pub job_id: JobId,
  /// Catalog model slug the job ran on (the path resource for a replay).
  #[schema(example = "openai_gpt_image_1_5")]
  pub model: String,
  pub prompt: String,
  /// Edit inputs, resolved to CDN URLs — `image_inputs` accepts these directly.
  pub image_inputs: Vec<String>,
  pub mask: Option<String>,
  /// Post-clamp fan-out (what actually ran).
  pub n: u8,
  pub size: Option<ApiImageSize>,
  pub quality: Option<ApiImageQuality>,
  pub output_format: Option<ApiOutputFormat>,
  pub input_fidelity: Option<ApiInputFidelity>,
  pub dry_run: bool,
}

/// Map a job row into the replayable wire shape (shared with the admin twin).
pub(crate) fn job_request_response(job: &db_queries::Job) -> ApiResult<JobRequestResponse> {
  let stored: StoredImageParams = serde_json::from_value(job.request_params.clone())
    .map_err(|e| ApiError::Internal(format!("stored params unreadable: {e}")))?;
  Ok(JobRequestResponse {
    job_id: job.id.clone(),
    model: stored.model,
    prompt: stored.prompt,
    image_inputs: stored.image_urls,
    mask: stored.mask_url,
    n: stored.n,
    size: stored.size.map(Into::into),
    quality: stored.quality.map(Into::into),
    output_format: stored.output_format.map(Into::into),
    input_fidelity: stored.input_fidelity.map(Into::into),
    dry_run: job.is_dry_run,
  })
}

/// The request that created this job — for "recreate" flows and request inspection.
#[utoipa::path(
  get, path = "/v1/jobs/{id}/request",
  tag = "generation",
  security(("session_cookie" = []), ("api_key" = [])),
  params(("id" = String, Path, description = "Job id (`job_…`)"), JobScopeParams),
  responses(
    (status = 200, description = "The originating request, replayable", body = JobRequestResponse),
    (status = 400, description = "Malformed job or account id"),
    (status = 404, description = "No such job in this account, or the caller is not a member")
  )
)]
pub async fn get_job_request_handler(
  State(state): State<AppState>,
  caller: Caller,
  Path(id): Path<String>,
  Query(scope): Query<JobScopeParams>,
) -> ApiResult<Json<JobRequestResponse>> {
  let job_id: JobId = id.parse().map_err(|_| ApiError::BadRequest("invalid job id".into()))?;
  let requested = scope.parsed_account_id()?;
  let account_id = resolve_caller_account(&state, &caller, requested.as_ref()).await?;
  let job = db_queries::jobs::find_job_for_account(&state.pool, &account_id, &job_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("job not found".into()))?;
  Ok(Json(job_request_response(&job)?))
}
