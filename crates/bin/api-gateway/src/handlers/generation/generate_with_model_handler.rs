//! `POST /v1/models/{slug}/generate` — the unified model-specific plane
//! (api_model_specific.md): the model is the path resource; modality is inferred from the
//! params (image inputs present = edit).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use enums::ApiShape;

use super::types::{session_enqueue_context, ImageParamsBody};
use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::JobBody;
use crate::generation::service::enqueue_image_job;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Enqueues an image job on this specific model — poll `GET /v1/jobs/{id}`. Same async
/// contract as the Any plane; the body omits `model` (it's the path).
#[utoipa::path(
  post, path = "/v1/models/{slug}/generate",
  tag = "generation",
  security(("session_cookie" = [])),
  params(("slug" = String, Path, description = "Catalog model slug, e.g. `openai_gpt_image_v2`")),
  request_body = ImageParamsBody,
  responses(
    (status = 201, description = "Job accepted and queued", body = JobBody),
    (status = 400, description = "Empty prompt, bad image refs, or invalid params"),
    (status = 404, description = "Unknown/unrouted model slug, unknown account, or not a member")
  )
)]
pub async fn generate_with_model_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(slug): Path<String>,
  Json(body): Json<ImageParamsBody>,
) -> ApiResult<(StatusCode, Json<JobBody>)> {
  if catalog::by_slug(&slug).is_none() {
    return Err(ApiError::NotFound(format!("unknown model: {slug}")));
  }
  let account_id = body.account_id.clone();
  let ctx = session_enqueue_context(&state, &auth, &account_id, ApiShape::Unified).await?;
  let stored = body.into_stored(&state, &slug).await?;
  let job = enqueue_image_job(&state, ctx, stored).await?;
  Ok((StatusCode::CREATED, Json(job)))
}
