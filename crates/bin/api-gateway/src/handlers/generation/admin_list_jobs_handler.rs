//! `GET /v1/admin/jobs` — staff-only recent jobs across ALL accounts, keyset-paginated.
//! Plain pagination, no polling — the admin surface is a record browser, not a monitor.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use ids::JobId;

use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::{job_bodies, JobBody};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct AdminListJobsParams {
  /// Keyset cursor: jobs older than this job id.
  pub after: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminJobItem {
  #[serde(flatten)]
  pub job: JobBody,
  /// Which account ran it (admin view spans accounts).
  #[schema(value_type = String, example = "acct_01k0…")]
  pub account_id: ids::AccountId,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminListJobsResponse {
  pub jobs: Vec<AdminJobItem>,
  #[schema(value_type = Option<String>)]
  pub next_after: Option<JobId>,
}

const PAGE_SIZE: i64 = 50;

/// Staff only. Newest first across all accounts.
#[utoipa::path(
  get, path = "/v1/admin/jobs",
  tag = "admin",
  security(("session_cookie" = [])),
  params(AdminListJobsParams),
  responses(
    (status = 200, description = "One page of recent jobs, all accounts", body = AdminListJobsResponse),
    (status = 400, description = "Malformed cursor"),
    (status = 403, description = "Caller is not staff")
  )
)]
pub async fn admin_list_jobs_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Query(params): Query<AdminListJobsParams>,
) -> ApiResult<Json<AdminListJobsResponse>> {
  if !auth.user.is_staff {
    return Err(ApiError::Forbidden("staff only".into()));
  }
  let after: Option<JobId> = match &params.after {
    Some(raw) => Some(raw.parse().map_err(|_| ApiError::BadRequest("invalid `after` cursor".into()))?),
    None => None,
  };
  let rows = db_queries::jobs::list_recent_jobs(&state.pool, after.as_ref(), PAGE_SIZE + 1).await?;
  let has_more = rows.len() as i64 > PAGE_SIZE;
  let page = &rows[..rows.len().min(PAGE_SIZE as usize)];

  let bodies = job_bodies(&state, page).await?;
  let jobs = bodies
    .into_iter()
    .zip(page)
    .map(|(job, row)| AdminJobItem { job, account_id: row.account_id.clone() })
    .collect();
  let next_after = has_more.then(|| page.last().map(|j| j.id.clone())).flatten();
  Ok(Json(AdminListJobsResponse { jobs, next_after }))
}
