use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use db_queries::gateway_requests::submission_limits as storage;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{access::ScopeQuery, routing_settings::managed_target};
use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthContext,
    state::AppState,
};

#[derive(Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(super) struct OrganizationLimitQuery {
    account_id: String,
}

impl From<OrganizationLimitQuery> for ScopeQuery {
    fn from(query: OrganizationLimitQuery) -> Self {
        Self {
            account_id: Some(query.account_id),
            project_id: None,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub(super) struct SubmissionLimit {
    requests_per_minute: i32,
    burst: i32,
    next_self_service_limit: Option<i32>,
    next_self_service_at: Option<DateTime<Utc>>,
    operator_managed: bool,
}

impl From<storage::Limit> for SubmissionLimit {
    fn from(limit: storage::Limit) -> Self {
        Self {
            requests_per_minute: limit.requests_per_minute,
            burst: limit.burst,
            next_self_service_limit: limit.next_self_service_limit,
            next_self_service_at: limit.next_self_service_at,
            operator_managed: limit.operator_managed,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct LimitIncrease {
    expected_requests_per_minute: i32,
    requested_requests_per_minute: i32,
}

#[utoipa::path(
    get, path = "/v1/rate-limits", tag = "native",
    summary = "Read the organization submission rate limit",
    security(("session_cookie" = [])), params(OrganizationLimitQuery),
    responses((status = 200, body = SubmissionLimit), (status = "default", response = super::RequestErrorResponse)),
)]
pub(super) async fn get(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationLimitQuery>,
) -> ApiResult<Json<SubmissionLimit>> {
    let (account, _) = managed_target(&state, &auth, query.into()).await?;
    Ok(Json(storage::get(&state.pool, &account).await?.into()))
}

#[utoipa::path(
    post, path = "/v1/rate-limits/increase", tag = "native",
    summary = "Request the next self-service submission limit tier",
    security(("session_cookie" = [])), params(OrganizationLimitQuery), request_body = LimitIncrease,
    responses((status = 200, body = SubmissionLimit), (status = "default", response = super::RequestErrorResponse)),
)]
pub(super) async fn increase(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationLimitQuery>,
    Json(input): Json<LimitIncrease>,
) -> ApiResult<Json<SubmissionLimit>> {
    let (account, _) = managed_target(&state, &auth, query.into()).await?;
    let outcome = storage::increase(
        &state.pool,
        &account,
        &auth.user.id,
        input.expected_requests_per_minute,
        input.requested_requests_per_minute,
    )
    .await?;
    match outcome.as_str() {
        "approved" => Ok(Json(storage::get(&state.pool, &account).await?.into())),
        "invalid_tier" => Err(ApiError::BadRequest(
            "request the next self-service tier, up to 600 requests per minute".into(),
        )),
        "operator_managed" => Err(ApiError::Conflict(
            "the operator manages this organization's submission limit".into(),
        )),
        "stale" => Err(ApiError::Conflict(
            "the submission limit changed. Read the current tier and retry".into(),
        )),
        "cooldown" => Err(ApiError::Conflict(
            "another increase is available 24 hours after the last approval".into(),
        )),
        _ => Err(ApiError::Internal(
            "unknown submission limit decision".into(),
        )),
    }
}
