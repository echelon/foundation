//! `POST /v1/any/image` — the Any plane (api_any.md): model is a body param, the schema is
//! the superset; unsupported params are pruned/coerced and reported, never rejected.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use enums::ApiShape;

use super::types::{enqueue_context, resolve_caller_account, ImageParamsBody};
use crate::error::ApiResult;
use crate::generation::job_response::JobBody;
use crate::generation::service::enqueue_image_job;
use crate::middleware::Caller;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AnyImageBody {
  /// Catalog slug of the image model to run, e.g. `openai_gpt_image_1_5`.
  #[schema(example = "openai_gpt_image_1_5")]
  pub model: String,
  #[serde(flatten)]
  pub params: ImageParamsBody,
}

/// Enqueues an image job (never held open) — poll `GET /v1/jobs/{id}`. With `n > 1` the
/// job fans out; outputs stream in per image and `progress` reports partial completion.
/// With `dry_run: true` nothing is sent to the provider: the job lands terminal
/// (`canceled` / `dry_run`) with the exact would-be payloads captured in the request log.
#[utoipa::path(
  post, path = "/v1/any/image",
  tag = "generation",
  security(("session_cookie" = []), ("api_key" = [])),
  request_body = AnyImageBody,
  responses(
    (status = 201, description = "Job accepted and queued", body = JobBody),
    (status = 400, description = "Unknown model, empty prompt, bad image refs, or invalid params"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn generate_any_image_handler(
  State(state): State<AppState>,
  caller: Caller,
  Json(body): Json<AnyImageBody>,
) -> ApiResult<(StatusCode, Json<JobBody>)> {
  let account_id = resolve_caller_account(&state, &caller, body.params.account_id.as_ref()).await?;
  let ctx = enqueue_context(&caller, &account_id, ApiShape::Any, body.params.dry_run);
  let stored = body.params.into_stored(&state, &account_id, &body.model).await?;
  let job = enqueue_image_job(&state, ctx, stored).await?;
  Ok((StatusCode::CREATED, Json(job)))
}
