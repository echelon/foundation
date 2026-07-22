//! `GET /v1/jobs/{id}` — the full Job resource: status, outputs-so-far (CDN URLs),
//! fan-out progress, failure info, and the price estimate.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use ids::{AccountId, JobId};

use super::types::resolve_caller_account;
use crate::error::{ApiError, ApiResult};
use crate::generation::job_response::JobBody;
use crate::generation::service::load_job_body;
use crate::middleware::Caller;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct JobScopeParams {
  /// The account the job belongs to. Required for session callers; API-key callers may
  /// omit it (the key names the account).
  pub account_id: Option<String>,
}

impl JobScopeParams {
  /// Parse the optional account id, rejecting malformed values before the caller gate.
  pub fn parsed_account_id(&self) -> ApiResult<Option<AccountId>> {
    match &self.account_id {
      Some(raw) => {
        Ok(Some(raw.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?))
      }
      None => Ok(None),
    }
  }
}

/// The poll target for enqueued generations. While a fanned-out job is `running`, `outputs`
/// grows as individual images finish — render partial results from it.
#[utoipa::path(
  get, path = "/v1/jobs/{id}",
  tag = "generation",
  security(("session_cookie" = []), ("api_key" = [])),
  params(("id" = String, Path, description = "Job id (`job_…`)"), JobScopeParams),
  responses(
    (status = 200, description = "The job", body = JobBody),
    (status = 400, description = "Malformed job or account id"),
    (status = 404, description = "No such job in this account, or the caller is not a member")
  )
)]
pub async fn get_job_handler(
  State(state): State<AppState>,
  caller: Caller,
  Path(id): Path<String>,
  Query(scope): Query<JobScopeParams>,
) -> ApiResult<Json<JobBody>> {
  let job_id: JobId = id.parse().map_err(|_| ApiError::BadRequest("invalid job id".into()))?;
  let requested = scope.parsed_account_id()?;
  let account_id = resolve_caller_account(&state, &caller, requested.as_ref()).await?;
  Ok(Json(load_job_body(&state, &account_id, &job_id).await?))
}
