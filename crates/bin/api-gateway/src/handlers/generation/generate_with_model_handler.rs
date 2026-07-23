//! `POST /v1/models/{slug}/generate` — the unified model-specific plane
//! (api_model_specific.md): the model is the path resource; the body schema is the
//! param set for that model's MEDIA TYPE (image vs video), so the handler looks up
//! the catalog spec first and then decodes the matching typed body.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use utoipa::ToSchema;

use enums::ApiShape;

use super::types::{enqueue_context, resolve_caller_account, ImageParamsBody};
use super::video_types::VideoParamsBody;
use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::JobBody;
use crate::generation::service::{enqueue_image_job, enqueue_video_job};
use crate::middleware::Caller;
use crate::state::AppState;

/// OpenAPI-only union: the unified plane's body is the image or the video param set —
/// which one is decided by the path model's media type, not by a tag in the body.
#[derive(ToSchema)]
#[allow(dead_code)]
pub enum GenerateParamsBody {
  Image(ImageParamsBody),
  Video(VideoParamsBody),
}

/// Decode a typed body or answer 400 with serde's message (what `Json<T>` would say).
pub(crate) fn decode_body<T: serde::de::DeserializeOwned>(bytes: &Bytes) -> ApiResult<T> {
  serde_json::from_slice(bytes).map_err(|e| ApiError::BadRequest(format!("invalid request body: {e}")))
}

/// Enqueues a generation job on this specific model — poll `GET /v1/jobs/{id}`. Same
/// async contract as the Any plane; the body omits `model` (it's the path) and its
/// schema follows the model's media type (image params or video params). Supports
/// `dry_run` (validate + price + log the exact payload, never send).
#[utoipa::path(
  post, path = "/v1/models/{slug}/generate",
  tag = "generation",
  security(("session_cookie" = []), ("api_key" = [])),
  params(("slug" = String, Path, description = "Catalog model slug, e.g. `openai_gpt_image_2`")),
  request_body = GenerateParamsBody,
  responses(
    (status = 201, description = "Job accepted and queued", body = JobBody),
    (status = 400, description = "Empty prompt, bad image refs, or invalid params"),
    (status = 404, description = "Unknown/unrouted model slug, unknown account, or not a member")
  )
)]
pub async fn generate_with_model_handler(
  State(state): State<AppState>,
  caller: Caller,
  Path(slug): Path<String>,
  bytes: Bytes,
) -> ApiResult<(StatusCode, Json<JobBody>)> {
  let spec = catalog::by_slug(&slug).ok_or_else(|| ApiError::NotFound(format!("unknown model: {slug}")))?;
  let job = match spec.media_type {
    catalog::MediaType::Image => {
      let body: ImageParamsBody = decode_body(&bytes)?;
      let account_id = resolve_caller_account(&state, &caller, body.account_id.as_ref()).await?;
      let ctx = enqueue_context(&caller, &account_id, ApiShape::Unified, body.dry_run);
      let stored = body.into_stored(&state, &account_id, &slug).await?;
      enqueue_image_job(&state, ctx, stored).await?
    }
    catalog::MediaType::Video => {
      let body: VideoParamsBody = decode_body(&bytes)?;
      let account_id = resolve_caller_account(&state, &caller, body.account_id.as_ref()).await?;
      let ctx = enqueue_context(&caller, &account_id, ApiShape::Unified, body.dry_run);
      let stored = body.into_stored(&state, &account_id, &slug).await?;
      enqueue_video_job(&state, ctx, stored).await?
    }
    _ => return Err(ApiError::BadRequest(format!("model {slug} is not generatable via this API yet"))),
  };
  Ok((StatusCode::CREATED, Json(job)))
}
