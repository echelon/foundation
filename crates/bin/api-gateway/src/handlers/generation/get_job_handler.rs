//! `GET /v1/jobs/{id}` — the full Job resource: status, outputs-so-far (CDN URLs),
//! fan-out progress, failure info, and the price estimate.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use ids::{AccountId, JobId};

use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::JobBody;
use crate::generation::service::load_job_body;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct JobScopeParams {
  /// The account the job belongs to.
  pub account_id: String,
}

/// The poll target for enqueued generations. While a fanned-out job is `running`, `outputs`
/// grows as individual images finish — render partial results from it.
#[utoipa::path(
  get, path = "/v1/jobs/{id}",
  tag = "generation",
  security(("session_cookie" = [])),
  params(("id" = String, Path, description = "Job id (`job_…`)"), JobScopeParams),
  responses(
    (status = 200, description = "The job", body = JobBody),
    (status = 400, description = "Malformed job or account id"),
    (status = 404, description = "No such job in this account, or the caller is not a member")
  )
)]
pub async fn get_job_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(id): Path<String>,
  Query(scope): Query<JobScopeParams>,
) -> ApiResult<Json<JobBody>> {
  let job_id: JobId = id.parse().map_err(|_| ApiError::BadRequest("invalid job id".into()))?;
  let account_id: AccountId =
    scope.account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  db_queries::members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;
  Ok(Json(load_job_body(&state, &account_id, &job_id).await?))
}
