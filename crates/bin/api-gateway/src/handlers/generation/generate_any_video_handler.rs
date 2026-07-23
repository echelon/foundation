//! `POST /v1/any/video` — the Any plane for VIDEO (api_any.md): model is a body param,
//! the schema is the superset; unsupported params are pruned/coerced and reported.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use enums::ApiShape;

use super::types::{enqueue_context, resolve_caller_account};
use super::video_types::VideoParamsBody;
use crate::error::ApiResult;
use crate::generation::job_response::JobBody;
use crate::generation::service::enqueue_video_job;
use crate::middleware::Caller;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AnyVideoBody {
  /// Catalog slug of the video model to run, e.g. `google_veo_3_1_fast`.
  #[schema(example = "google_veo_3_1_fast")]
  pub model: String,
  #[serde(flatten)]
  pub params: VideoParamsBody,
}

/// Enqueues a video job (never held open) — poll `GET /v1/jobs/{id}`. Veo runs as an
/// async provider operation: the outbound job starts it, the polling job lands results.
/// With `n > 1` the job fans out; outputs stream in per video and `progress` reports
/// partial completion. `dry_run: true` prices + logs the exact payloads, never sends.
#[utoipa::path(
  post, path = "/v1/any/video",
  tag = "generation",
  security(("session_cookie" = []), ("api_key" = [])),
  request_body = AnyVideoBody,
  responses(
    (status = 201, description = "Job accepted and queued", body = JobBody),
    (status = 400, description = "Unknown model, empty prompt, bad image ref, or invalid params"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn generate_any_video_handler(
  State(state): State<AppState>,
  caller: Caller,
  Json(body): Json<AnyVideoBody>,
) -> ApiResult<(StatusCode, Json<JobBody>)> {
  let account_id = resolve_caller_account(&state, &caller, body.params.account_id.as_ref()).await?;
  let ctx = enqueue_context(&caller, &account_id, ApiShape::Any, body.params.dry_run);
  let stored = body.params.into_stored(&state, &account_id, &body.model).await?;
  let job = enqueue_video_job(&state, ctx, stored).await?;
  Ok((StatusCode::CREATED, Json(job)))
}
