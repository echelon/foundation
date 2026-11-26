use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use db_queries::{
    account_audit_logs,
    gateway_projects::{self, ProjectAccess, ProjectFilter, ProjectGrant},
};
use enums::{AuditEvent, MemberRole};
use ids::{AccountId, GatewayProjectId, UserId};
use serde::{Deserialize, Serialize};

use super::{
    access::{parse_id, session_projects},
    requests::validate_client_ref,
};
use crate::{
    authz::require_account_role,
    error::{ApiError, ApiResult},
    middleware::AuthContext,
    state::AppState,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationQuery {
    account_id: String,
}

impl OrganizationQuery {
    pub fn new(account_id: String) -> Self {
        Self { account_id }
    }

    pub fn account(&self) -> ApiResult<AccountId> {
        parse_id(&self.account_id, "account_id")
    }
}

#[derive(Serialize)]
pub struct ProjectView {
    id: GatewayProjectId,
    name: String,
    is_default: bool,
    role: String,
    client_ref: Option<String>,
}

impl From<ProjectAccess> for ProjectView {
    fn from(row: ProjectAccess) -> Self {
        Self {
            id: row.id,
            name: row.name,
            is_default: row.is_default,
            role: row.role,
            client_ref: row.client_ref,
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
) -> ApiResult<Json<Vec<ProjectView>>> {
    let account = query.account()?;
    let projects = session_projects(&state.pool, &auth, &account, None, false).await?;
    Ok(Json(projects.into_iter().map(ProjectView::from).collect()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProject {
    name: String,
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Json(input): Json<CreateProject>,
) -> ApiResult<(StatusCode, Json<ProjectView>)> {
    let account = query.account()?;
    require_account_role(&state.pool, &auth.user.id, &account, MemberRole::ADMINS).await?;
    let name = validate_name(&input.name)?;
    let id = GatewayProjectId::generate();
    let mut tx = state.pool.begin().await?;
    gateway_projects::create_project(&mut *tx, &account, &id, name).await?;
    audit(
        &mut tx,
        &account,
        &auth.user.id,
        AuditEvent::GatewayProjectCreated,
        "gateway_project",
        id.as_str(),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(ProjectView {
            id,
            name: name.to_owned(),
            is_default: false,
            role: "admin".into(),
            client_ref: None,
        }),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum GrantRole {
    Developer,
    BillingViewer,
    ClientViewer,
}

impl GrantRole {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Developer => "developer",
            Self::BillingViewer => "billing_viewer",
            Self::ClientViewer => "client_viewer",
        }
    }
}

#[derive(Serialize)]
pub struct GrantView {
    user_id: UserId,
    role: String,
    client_ref: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantInput {
    role: GrantRole,
    client_ref: Option<String>,
}

pub async fn grants(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path(raw): Path<String>,
) -> ApiResult<Json<Vec<GrantView>>> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    let rows = gateway_projects::list_grants(&state.pool, &account, &project.id).await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| GrantView {
                user_id: row.user_id,
                role: row.role,
                client_ref: row.client_ref,
            })
            .collect(),
    ))
}

pub async fn set_grant(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, user)): Path<(String, String)>,
    Json(input): Json<GrantInput>,
) -> ApiResult<StatusCode> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    if project.is_default {
        return Err(ApiError::BadRequest(
            "the default project uses organization membership".into(),
        ));
    }
    let user = parse_id(&user, "user_id")?;
    let membership = db_queries::members::find_membership(&state.pool, &account, &user)
        .await?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;
    validate_client_ref(input.client_ref.as_deref())?;
    if matches!(input.role, GrantRole::ClientViewer) != input.client_ref.is_some() {
        return Err(ApiError::BadRequest(
            "client_viewer requires a client_ref; other roles have project-wide access".into(),
        ));
    }

    let mut tx = state.pool.begin().await?;
    gateway_projects::put_grant(
        &mut *tx,
        ProjectGrant {
            account: &account,
            project: &project.id,
            membership: &membership.id,
            role: input.role.as_str(),
            client_ref: input.client_ref.as_deref(),
        },
    )
    .await?;
    audit(
        &mut tx,
        &account,
        &auth.user.id,
        AuditEvent::GatewayProjectGrantChanged,
        "gateway_project",
        project.id.as_str(),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_grant(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, user)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    let user = parse_id(&user, "user_id")?;
    let membership = db_queries::members::find_membership(&state.pool, &account, &user)
        .await?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;
    let mut tx = state.pool.begin().await?;
    if gateway_projects::delete_grant(&mut *tx, &account, &project.id, &membership.id).await? {
        audit(
            &mut tx,
            &account,
            &auth.user.id,
            AuditEvent::GatewayProjectGrantChanged,
            "gateway_project",
            project.id.as_str(),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn managed_project(
    state: &AppState,
    auth: &AuthContext,
    query: &OrganizationQuery,
    raw: &str,
) -> ApiResult<(AccountId, ProjectAccess)> {
    let account = query.account()?;
    require_account_role(&state.pool, &auth.user.id, &account, MemberRole::ADMINS).await?;
    let project = parse_id(&raw, "project_id")?;
    let mut rows = gateway_projects::list_projects(
        &state.pool,
        &account,
        ProjectFilter {
            project: Some(&project),
            admin: true,
            ..Default::default()
        },
    )
    .await?;
    Ok((
        account,
        rows.pop()
            .ok_or_else(|| ApiError::NotFound("project not found".into()))?,
    ))
}

pub fn validate_name(name: &str) -> ApiResult<&str> {
    let name = name.trim();
    if !(1..=80).contains(&name.chars().count()) || name.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "name must be 1–80 characters without control characters".into(),
        ));
    }
    Ok(name)
}

pub async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &AccountId,
    actor: &UserId,
    event: AuditEvent,
    target_type: &str,
    target_id: &str,
) -> ApiResult<()> {
    account_audit_logs::create_account_audit_log(
        &mut **tx,
        account_audit_logs::NewAccountAuditLog {
            account_id: account,
            actor_user_id: Some(actor),
            user_id: None,
            event,
            target_type: Some(target_type),
            target_id: Some(target_id),
            data: None,
            ip: None,
        },
    )
    .await?;
    Ok(())
}
