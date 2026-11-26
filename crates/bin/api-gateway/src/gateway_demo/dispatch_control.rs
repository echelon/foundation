use axum::{
    extract::{Query, State},
    Json,
};
use db_queries::{
    account_audit_logs::{self, NewAccountAuditLog},
    gateway_dispatch_controls,
};
use enums::{AuditEvent, MemberRole};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{catalog::Provider, projects::OrganizationQuery};
use crate::{
    authz::require_account_role,
    error::{ApiError, ApiResult},
    middleware::AuthContext,
    state::AppState,
};

#[derive(Serialize, ToSchema)]
pub(super) struct DispatchControl {
    revision: i32,
    pause_all: bool,
    paused_providers: Vec<Provider>,
}

impl TryFrom<gateway_dispatch_controls::DispatchControl> for DispatchControl {
    type Error = ApiError;

    fn try_from(row: gateway_dispatch_controls::DispatchControl) -> ApiResult<Self> {
        let paused_providers = row
            .paused_providers
            .iter()
            .map(|provider| {
                provider
                    .parse()
                    .map_err(|_| ApiError::Internal("invalid stored dispatch control".into()))
            })
            .collect::<ApiResult<_>>()?;
        Ok(Self {
            revision: row.revision,
            pause_all: row.pause_all,
            paused_providers,
        })
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct UpdateDispatchControl {
    expected_revision: i32,
    pause_all: bool,
    paused_providers: Vec<Provider>,
}

#[utoipa::path(
    get, path = "/v1/operations/dispatch", tag = "meta", security(("session_cookie" = [])),
    params(("account_id" = String, Query, description = "Organization to inspect.")),
    responses((status = 200, description = "Current dispatch controls.", body = DispatchControl)),
)]
pub(super) async fn get(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
) -> ApiResult<Json<DispatchControl>> {
    let account = query.account()?;
    require_account_role(&state.pool, &auth.user.id, &account, MemberRole::ADMINS).await?;
    Ok(Json(
        gateway_dispatch_controls::read(&state.pool, &account)
            .await?
            .try_into()?,
    ))
}

#[utoipa::path(
    put, path = "/v1/operations/dispatch", tag = "meta", security(("session_cookie" = [])),
    params(("account_id" = String, Query, description = "Organization to update.")),
    request_body = UpdateDispatchControl,
    responses(
        (status = 200, description = "Saved dispatch controls.", body = DispatchControl),
        (status = 409, description = "The dispatch control revision changed."),
    ),
)]
pub(super) async fn set(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Json(mut input): Json<UpdateDispatchControl>,
) -> ApiResult<Json<DispatchControl>> {
    let account = query.account()?;
    require_account_role(&state.pool, &auth.user.id, &account, MemberRole::ADMINS).await?;
    if !(0..i32::MAX).contains(&input.expected_revision) {
        return Err(ApiError::BadRequest(
            "invalid dispatch control revision".into(),
        ));
    }
    if input.paused_providers.len() > 64 {
        return Err(ApiError::BadRequest(
            "select each provider at most once".into(),
        ));
    }
    input.paused_providers.sort_unstable();
    if input
        .paused_providers
        .windows(2)
        .any(|pair| pair[0] == pair[1])
    {
        return Err(ApiError::BadRequest(
            "select each provider at most once".into(),
        ));
    }
    let providers: Vec<_> = input
        .paused_providers
        .iter()
        .map(|provider| provider.as_str().to_owned())
        .collect();
    let mut tx = state
        .pool
        .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED")
        .await?;
    let control: DispatchControl = gateway_dispatch_controls::save(
        &mut tx,
        &account,
        input.expected_revision,
        input.pause_all,
        &providers,
    )
    .await?
    .ok_or_else(|| ApiError::Conflict("dispatch controls changed; reload and try again".into()))?
    .try_into()?;
    account_audit_logs::create_account_audit_log(
        &mut *tx,
        NewAccountAuditLog {
            account_id: &account,
            actor_user_id: Some(&auth.user.id),
            user_id: None,
            event: AuditEvent::GatewayDispatchControlChanged,
            target_type: Some("gateway_dispatch_control"),
            target_id: Some(account.as_str()),
            data: Some(serde_json::json!(&control)),
            ip: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(control))
}
