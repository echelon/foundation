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
use super::video_types::{ApiVideoAspectRatio, ApiVideoResolution};
use crate::error::{ApiError, ApiResult};
use crate::generation::stored_params::{StoredImageParams, StoredVideoParams};
use crate::middleware::Caller;
use crate::state::AppState;

/// A job's originating request, replayable against the generate endpoints. Image jobs
/// fill the image params; video jobs fill the video params — `media_type` says which.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobRequestResponse {
  #[schema(value_type = String, example = "job_01k0…")]
  pub job_id: JobId,
  /// Catalog model slug the job ran on (the path resource for a replay).
  #[schema(example = "openai_gpt_image_1_5")]
  pub model: String,
  /// `image` or `video` — which param set below is populated.
  #[schema(example = "image")]
  pub media_type: String,
  pub prompt: String,
  /// Post-clamp fan-out (what actually ran).
  pub n: u8,
  pub dry_run: bool,

  // ── image params (media_type = image) ──
  /// Edit inputs, resolved to CDN URLs — `image_inputs` accepts these directly.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub image_inputs: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub mask: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub size: Option<ApiImageSize>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub quality: Option<ApiImageQuality>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output_format: Option<ApiOutputFormat>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub input_fidelity: Option<ApiInputFidelity>,

  // ── video params (media_type = video) ──
  #[serde(skip_serializing_if = "Option::is_none")]
  pub negative_prompt: Option<String>,
  /// i2v conditioning frame, resolved to a CDN URL — `image_input` accepts it directly.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image_input: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolution: Option<ApiVideoResolution>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub duration_seconds: Option<u8>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<ApiVideoAspectRatio>,
}

impl JobRequestResponse {
  fn base(job: &db_queries::Job, model: String, prompt: String, n: u8) -> Self {
    Self {
      job_id: job.id.clone(),
      model,
      media_type: job.media_type.clone(),
      prompt,
      n,
      dry_run: job.is_dry_run,
      image_inputs: Vec::new(),
      mask: None,
      size: None,
      quality: None,
      output_format: None,
      input_fidelity: None,
      negative_prompt: None,
      image_input: None,
      resolution: None,
      duration_seconds: None,
      aspect_ratio: None,
    }
  }
}

/// Map a job row into the replayable wire shape (shared with the admin twin).
pub(crate) fn job_request_response(job: &db_queries::Job) -> ApiResult<JobRequestResponse> {
  let unreadable = |e: serde_json::Error| ApiError::Internal(format!("stored params unreadable: {e}"));
  if job.media_type == catalog::MediaType::Video.as_str() {
    let stored: StoredVideoParams = serde_json::from_value(job.request_params.clone()).map_err(unreadable)?;
    let mut body = JobRequestResponse::base(job, stored.model, stored.prompt, stored.n);
    body.negative_prompt = stored.negative_prompt;
    body.image_input = stored.image_url;
    body.resolution = stored.resolution.map(Into::into);
    body.duration_seconds = stored.duration.map(|d| d.seconds() as u8);
    body.aspect_ratio = stored.aspect_ratio.map(Into::into);
    return Ok(body);
  }
  let stored: StoredImageParams = serde_json::from_value(job.request_params.clone()).map_err(unreadable)?;
  let mut body = JobRequestResponse::base(job, stored.model, stored.prompt, stored.n);
  body.image_inputs = stored.image_urls;
  body.mask = stored.mask_url;
  body.size = stored.size.map(Into::into);
  body.quality = stored.quality.map(Into::into);
  body.output_format = stored.output_format.map(Into::into);
  body.input_fidelity = stored.input_fidelity.map(Into::into);
  Ok(body)
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
