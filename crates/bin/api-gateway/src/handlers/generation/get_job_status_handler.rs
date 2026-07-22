//! `GET /v1/jobs/{id}/status` — the lightweight poll (jobs.md): status + progress only,
//! no outputs join.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use ids::JobId;

use super::get_job_handler::JobScopeParams;
use super::types::resolve_caller_account;
use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::JobProgressBody;
use crate::middleware::Caller;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct JobStatusResponse {
  #[schema(value_type = String, example = "job_01k0…")]
  pub id: JobId,
  #[schema(value_type = String, example = "running")]
  pub status: enums::JobStatus,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub progress: Option<JobProgressBody>,
}

/// Cheap poll — prefer this on tight intervals, and fetch `GET /v1/jobs/{id}` when
/// `status` goes terminal or `progress.completed` moves.
#[utoipa::path(
  get, path = "/v1/jobs/{id}/status",
  tag = "generation",
  security(("session_cookie" = []), ("api_key" = [])),
  params(("id" = String, Path, description = "Job id (`job_…`)"), JobScopeParams),
  responses(
    (status = 200, description = "Status + fan-out progress", body = JobStatusResponse),
    (status = 400, description = "Malformed job or account id"),
    (status = 404, description = "No such job in this account, or the caller is not a member")
  )
)]
pub async fn get_job_status_handler(
  State(state): State<AppState>,
  caller: Caller,
  Path(id): Path<String>,
  Query(scope): Query<JobScopeParams>,
) -> ApiResult<Json<JobStatusResponse>> {
  let job_id: JobId = id.parse().map_err(|_| ApiError::BadRequest("invalid job id".into()))?;
  let requested = scope.parsed_account_id()?;
  let account_id = resolve_caller_account(&state, &caller, requested.as_ref()).await?;
  let job = db_queries::jobs::find_job_for_account(&state.pool, &account_id, &job_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("job not found".into()))?;
  let slices = db_queries::job_slices::list_slices_for_job(&state.pool, &job.id).await?;
  let progress = (!slices.is_empty()).then(|| {
    let terminal = slices.iter().filter(|s| s.status.is_terminal()).count() as u32;
    JobProgressBody {
      completed: terminal,
      total: slices.len() as u32,
      succeeded: slices.iter().filter(|s| s.status == enums::JobSliceStatus::Succeeded).count() as u32,
      failed: slices.iter().filter(|s| s.status == enums::JobSliceStatus::Failed).count() as u32,
    }
  });
  Ok(Json(JobStatusResponse { id: job.id, status: job.status, progress }))
}
