//! `POST /v1/any/image` — the Any plane (api_any.md): model is a body param, the schema is
//! the superset; unsupported params are pruned/coerced and reported, never rejected.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use enums::ApiShape;

use super::types::{session_enqueue_context, ImageParamsBody};
use crate::error::ApiResult;
use crate::generation::job_response::JobBody;
use crate::generation::service::enqueue_image_job;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AnyImageBody {
  /// Catalog slug of the image model to run, e.g. `openai_gpt_image_v1_5`.
  #[schema(example = "openai_gpt_image_v1_5")]
  pub model: String,
  #[serde(flatten)]
  pub params: ImageParamsBody,
}

/// Enqueues an image job (never held open) — poll `GET /v1/jobs/{id}`. With `n > 1` the
/// job fans out; outputs stream in per image and `progress` reports partial completion.
#[utoipa::path(
  post, path = "/v1/any/image",
  tag = "generation",
  security(("session_cookie" = [])),
  request_body = AnyImageBody,
  responses(
    (status = 201, description = "Job accepted and queued", body = JobBody),
    (status = 400, description = "Unknown model, empty prompt, bad image refs, or invalid params"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn generate_any_image_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Json(body): Json<AnyImageBody>,
) -> ApiResult<(StatusCode, Json<JobBody>)> {
  let account_id = body.params.account_id.clone();
  let ctx = session_enqueue_context(&state, &auth, &account_id, ApiShape::Any).await?;
  let stored = body.params.into_stored(&state, &body.model).await?;
  let job = enqueue_image_job(&state, ctx, stored).await?;
  Ok((StatusCode::CREATED, Json(job)))
}
