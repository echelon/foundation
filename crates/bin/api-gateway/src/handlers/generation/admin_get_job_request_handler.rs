//! `GET /v1/admin/jobs/{id}/request` — staff-only twin of `/v1/jobs/{id}/request`: the
//! admin surface spans accounts, so it can't pass the membership gate the customer
//! endpoint enforces. Same typed, replayable response shape.

use axum::extract::{Path, State};
use axum::Json;

use ids::JobId;

use super::get_job_request_handler::{job_request_response, JobRequestResponse};
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Staff only. The originating request behind any account's job.
#[utoipa::path(
  get, path = "/v1/admin/jobs/{id}/request",
  tag = "admin",
  security(("session_cookie" = [])),
  params(("id" = String, Path, description = "Job id (`job_…`)")),
  responses(
    (status = 200, description = "The originating request, replayable", body = JobRequestResponse),
    (status = 400, description = "Malformed job id"),
    (status = 403, description = "Caller is not staff"),
    (status = 404, description = "No such job")
  )
)]
pub async fn admin_get_job_request_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(id): Path<String>,
) -> ApiResult<Json<JobRequestResponse>> {
  if !auth.user.is_staff {
    return Err(ApiError::Forbidden("staff only".into()));
  }
  let job_id: JobId = id.parse().map_err(|_| ApiError::BadRequest("invalid job id".into()))?;
  let job = db_queries::jobs::find_job(&state.pool, &job_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("job not found".into()))?;
  Ok(Json(job_request_response(&job)?))
}
