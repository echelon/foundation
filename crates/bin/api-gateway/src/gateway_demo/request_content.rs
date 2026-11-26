use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use db_queries::gateway_requests::{remove_content, ContentRemoval, RequestScope};
use enums::AuditEvent;
use serde::Serialize;

use super::handlers::parse_request_id;
use super::projects::{audit, managed_project, OrganizationQuery};
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct RemovedContent {
    content_removed_at: DateTime<Utc>,
}

#[utoipa::path(
    delete, path = "/v1/projects/{project}/requests/{request}/content",
    tag = "generation", security(("session_cookie" = [])),
    params(
        ("project" = String, Path, description = "Project ID"),
        ("request" = String, Path, description = "Request ID"),
        ("account_id" = String, Query, description = "Organization account ID"),
    ),
    responses(
        (status = 200, description = "Stored content removed. The first removal emits a scoped request.content_removed export event.", body = RemovedContent),
        (status = 401, description = "Administrator session required"),
        (status = 403, description = "Administrator role required"),
        (status = 404, description = "Project or request unavailable"),
        (status = 409, description = "Only terminal provider requests are eligible"),
    ),
)]
pub(crate) async fn delete(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((project, raw)): Path<(String, String)>,
) -> ApiResult<Json<RemovedContent>> {
    let (account, project) = managed_project(&state, &auth, &query, &project).await?;
    let request = parse_request_id(&raw)?;
    let scope = RequestScope {
        account: &account,
        project: &project.id,
        client_ref: None,
    };
    let mut tx = state.pool.begin().await?;
    let at = match remove_content(&mut tx, scope, &request).await? {
        ContentRemoval::Missing => {
            return Err(ApiError::NotFound(
                "request not found in this project".into(),
            ));
        }
        ContentRemoval::Ineligible => {
            return Err(ApiError::Conflict(
                "content removal requires a succeeded, failed, or canceled provider request".into(),
            ));
        }
        ContentRemoval::Removed { at, first } => {
            if first {
                audit(
                    &mut tx,
                    &account,
                    &auth.user.id,
                    AuditEvent::GatewayRequestContentRemoved,
                    "gateway_request",
                    request.as_str(),
                )
                .await?;
            }
            at
        }
    };
    tx.commit().await?;
    Ok(Json(RemovedContent {
        content_removed_at: at,
    }))
}
