//! `GET /v1/jobs` — the account's job history (jobs.md: "this table IS the customer spend
//! list"), keyset-paginated newest-first.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use ids::{AccountId, JobId};

use super::types::resolve_caller_account;
use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::{job_bodies, JobBody};
use crate::middleware::Caller;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListJobsParams {
  /// The account whose jobs to list. Required for session callers; API-key callers may
  /// omit it (the key names the account).
  pub account_id: Option<String>,
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
  security(("session_cookie" = []), ("api_key" = [])),
  params(ListJobsParams),
  responses(
    (status = 200, description = "One page of jobs", body = ListJobsResponse),
    (status = 400, description = "Malformed account id or cursor"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn list_jobs_handler(
  State(state): State<AppState>,
  caller: Caller,
  Query(params): Query<ListJobsParams>,
) -> ApiResult<Json<ListJobsResponse>> {
  let requested: Option<AccountId> = match &params.account_id {
    Some(raw) => {
      Some(raw.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?)
    }
    None => None,
  };
  let account_id = resolve_caller_account(&state, &caller, requested.as_ref()).await?;
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

  let jobs = job_bodies(&state, page).await?;
  let next_after = has_more.then(|| page.last().map(|j| j.id.clone())).flatten();
  Ok(Json(ListJobsResponse { jobs, next_after }))
}
