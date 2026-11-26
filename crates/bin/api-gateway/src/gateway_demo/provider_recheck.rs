use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use db_queries::gateway_requests::{recheck_provider, Recheck, RequestScope};
use enums::AuditEvent;
use serde::Deserialize;
use utoipa::ToSchema;

use super::{
    handlers::parse_request_id,
    projects::{audit, managed_project, OrganizationQuery},
};
use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthContext,
    state::AppState,
};

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct RecheckRequest {}

#[utoipa::path(
    post, path = "/v1/projects/{project}/requests/{request}/recheck",
    tag = "native", security(("session_cookie" = [])),
    params(
        ("project" = String, Path, description = "Project ID"),
        ("request" = String, Path, description = "Request ID"),
        ("account_id" = String, Query, description = "Organization account ID"),
    ),
    request_body = RecheckRequest,
    responses(
        (status = 202, description = "A status and result recheck was scheduled for the original provider job."),
        (status = 401, description = "Administrator session required."),
        (status = 403, description = "Administrator role required."),
        (status = 404, description = "Project or request unavailable."),
        (status = 409, description = "The request is ineligible or a recheck has already been scheduled."),
    ),
)]
pub(super) async fn schedule(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((project, raw)): Path<(String, String)>,
    Json(_body): Json<RecheckRequest>,
) -> ApiResult<StatusCode> {
    let (account, project) = managed_project(&state, &auth, &query, &project).await?;
    let request = parse_request_id(&raw)?;
    let scope = RequestScope {
        account: &account,
        project: &project.id,
        client_ref: None,
    };
    let mut tx = state.pool.begin_with("BEGIN ISOLATION LEVEL READ COMMITTED").await?;
    match recheck_provider(&mut tx, scope, &request).await? {
        Recheck::Missing => return Err(ApiError::NotFound("request not found in this project".into())),
        Recheck::Ineligible => return Err(ApiError::Conflict(
            "recheck requires a polling timeout within 24 hours, an available original credential, and no prior recheck".into(),
        )),
        Recheck::Scheduled => audit(
            &mut tx, &account, &auth.user.id, AuditEvent::GatewayProviderRecheckScheduled,
            "gateway_request", request.as_str(),
        ).await?,
    }
    tx.commit().await?;
    Ok(StatusCode::ACCEPTED)
}
