use axum::extract::{Path, State};
use axum::Json;
use db_queries::gateway_requests::find_request_location;
use ids::{AccountId, GatewayProjectId, GatewayRequestId};
use serde::Serialize;

use super::access::{self, Permission};
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Serialize)]
pub(super) struct LocatedRequest {
    request_id: GatewayRequestId,
    account_id: AccountId,
    project_id: GatewayProjectId,
}

pub(super) async fn get(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(raw): Path<String>,
) -> ApiResult<Json<LocatedRequest>> {
    let id = access::parse_id(&raw, "request ID")?;
    let missing = || ApiError::NotFound("Request not found in your accessible projects.".into());
    let location = find_request_location(&state.pool, &id).await?.ok_or_else(missing)?;
    let projects = access::session_projects(
        &state.pool,
        &auth,
        &location.account_id,
        Some(&location.project_id),
        false,
    ).await.map_err(|err| match err {
        ApiError::NotFound(_) | ApiError::Forbidden(_) => missing(),
        other => other,
    })?;

    if !projects.iter().any(|project| {
        access::role_allows(&project.role, Permission::ReadRequests)
            && project.client_ref.as_deref().is_none_or(|bound| location.client_ref.as_deref() == Some(bound))
    }) {
        return Err(missing());
    }
    Ok(Json(LocatedRequest {
        request_id: id,
        account_id: location.account_id,
        project_id: location.project_id,
    }))
}
