//! `GET /v1/jobs` — the account's job history (jobs.md: "this table IS the customer spend
//! list"), keyset-paginated newest-first.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use ids::{AccountId, JobId};

use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::{job_body, JobBody};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListJobsParams {
  /// The account whose jobs to list.
  pub account_id: String,
  /// Filter to one catalog model slug.
  pub model: Option<String>,
  /// Keyset cursor: return jobs older than this job id (from the previous page's last row).
  pub after: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListJobsResponse {
  pub jobs: Vec<JobBody>,
  /// Pass as `after` to fetch the next (older) page; absent on the last page.
  #[schema(value_type = Option<String>)]
  pub next_after: Option<JobId>,
}

const PAGE_SIZE: i64 = 25;

/// Newest first, 25 per page. Listing rows include outputs + progress, so a dashboard
/// history view needs no extra per-job fetches.
#[utoipa::path(
  get, path = "/v1/jobs",
  tag = "generation",
  security(("session_cookie" = [])),
  params(ListJobsParams),
  responses(
    (status = 200, description = "One page of jobs", body = ListJobsResponse),
    (status = 400, description = "Malformed account id or cursor"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn list_jobs_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Query(params): Query<ListJobsParams>,
) -> ApiResult<Json<ListJobsResponse>> {
  let account_id: AccountId =
    params.account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  db_queries::members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;
  let after: Option<JobId> = match &params.after {
    Some(raw) => Some(raw.parse().map_err(|_| ApiError::BadRequest("invalid `after` cursor".into()))?),
    None => None,
  };

  let rows = db_queries::jobs::list_jobs_for_account(
    &state.pool,
    &account_id,
    db_queries::jobs::JobListFilter { model: params.model.as_deref(), after: after.as_ref() },
    PAGE_SIZE + 1,
  )
  .await?;
  let has_more = rows.len() as i64 > PAGE_SIZE;
  let page = &rows[..rows.len().min(PAGE_SIZE as usize)];

  let mut jobs = Vec::with_capacity(page.len());
  for job in page {
    let slices = db_queries::job_slices::list_slices_for_job(&state.pool, &job.id).await?;
    let outputs = db_queries::outputs::list_outputs_with_files_for_job(&state.pool, &job.id).await?;
    jobs.push(job_body(&state, job, &slices, &outputs));
  }
  let next_after = has_more.then(|| page.last().map(|j| j.id.clone())).flatten();
  Ok(Json(ListJobsResponse { jobs, next_after }))
}
