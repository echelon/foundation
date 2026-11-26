use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use db_queries::gateway_projects::{self, NewPrincipal, NewProjectKey};
use enums::AuditEvent;
use ids::{AccountId, GatewayPrincipalId, GatewayProjectId, GatewayProjectKeyId};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};

use super::{
    access::{parse_id, Permission, PROJECT_KEY_PREFIX},
    projects::{audit, managed_project, validate_name, OrganizationQuery},
    requests::validate_client_ref,
};
use crate::{
    auth::service::sha256_hex,
    error::{ApiError, ApiResult},
    middleware::AuthContext,
    state::AppState,
};

#[derive(Serialize)]
struct Principal {
    id: GatewayPrincipalId,
    name: String,
    scopes: Vec<String>,
    client_ref: Option<String>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct KeyMetadata {
    id: GatewayProjectKeyId,
    principal_id: GatewayPrincipalId,
    key_prefix: String,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct Credentials {
    attached_principal_id: Option<GatewayPrincipalId>,
    principals: Vec<Principal>,
    keys: Vec<KeyMetadata>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path(raw): Path<String>,
) -> ApiResult<Json<Credentials>> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    let mut tx = state.pool.begin().await?;
    db_queries::gateway_requests::read_snapshot(&mut *tx).await?;
    let attached_principal_id = gateway_projects::attached_principal(&mut *tx, &account, &project.id).await?;
    let principals = gateway_projects::list_principals(&mut *tx, &account, &project.id).await?;
    let keys = gateway_projects::list_keys(&mut *tx, &account, &project.id).await?;
    tx.commit().await?;
    Ok(Json(Credentials {
        attached_principal_id,
        principals: principals
            .into_iter()
            .map(|row| Principal {
                id: row.id,
                name: row.name,
                scopes: row.scopes,
                client_ref: row.client_ref,
                revoked_at: row.revoked_at,
            })
            .collect(),
        keys: keys
            .into_iter()
            .map(|row| KeyMetadata {
                id: row.id,
                principal_id: row.principal_id,
                key_prefix: row.key_prefix,
                expires_at: row.expires_at,
                revoked_at: row.revoked_at,
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrincipalInput {
    name: String,
    scopes: Vec<Permission>,
    client_ref: Option<String>,
    expires_in_days: Option<u16>,
    #[serde(default)]
    attach: Option<bool>,
    #[serde(default)]
    bootstrap_default: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewKeyInput {
    expires_in_days: Option<u16>,
}

#[derive(Serialize)]
pub struct IssuedKey {
    id: GatewayProjectKeyId,
    principal_id: GatewayPrincipalId,
    key_prefix: String,
    expires_at: DateTime<Utc>,
    token: String,
}

pub async fn create_principal(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path(raw): Path<String>,
    Json(input): Json<PrincipalInput>,
) -> ApiResult<(StatusCode, Json<IssuedKey>)> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    let name = validate_name(&input.name)?;
    validate_client_ref(input.client_ref.as_deref())?;
    let mut scopes: Vec<_> = input
        .scopes
        .into_iter()
        .map(|scope| scope.as_str().to_owned())
        .collect();
    scopes.sort_unstable();
    scopes.dedup();
    if scopes.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one permission is required".into(),
        ));
    }
    let principal = GatewayPrincipalId::generate();
    let mut tx = state.pool.begin().await?;
    let attached = gateway_projects::lock_project_key(&mut *tx, &account, &project.id).await?;
    if input.bootstrap_default && (!project.is_default || name != "Default" || attached.is_some()) {
        return Err(ApiError::Conflict("Refresh project settings before creating the Default key.".into()));
    }
    gateway_projects::create_principal(
        &mut *tx,
        NewPrincipal {
            id: &principal,
            account: &account,
            project: &project.id,
            name,
            scopes: &scopes,
            client_ref: input.client_ref.as_deref(),
        },
    )
    .await?;
    let issued = issue_key(
        &mut tx,
        &account,
        &project.id,
        principal,
        input.expires_in_days,
    )
    .await?;
    audit(
        &mut tx,
        &account,
        &auth.user.id,
        AuditEvent::ApiKeyCreated,
        "gateway_project_key",
        issued.id.as_str(),
    )
    .await?;
    if input.bootstrap_default || input.attach.unwrap_or(attached.is_none()) {
        attach(&mut tx, &account, &project.id, &issued.principal_id, &auth).await?;
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(issued)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachInput {
    principal_id: String,
}

pub async fn select_attached(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path(raw): Path<String>,
    Json(input): Json<AttachInput>,
) -> ApiResult<StatusCode> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    let principal = parse_id(&input.principal_id, "principal_id")?;
    let mut tx = state.pool.begin().await?;
    gateway_projects::lock_project_key(&mut *tx, &account, &project.id).await?;
    attach(&mut tx, &account, &project.id, &principal, &auth).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn attach(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &AccountId,
    project: &GatewayProjectId,
    principal: &GatewayPrincipalId,
    auth: &AuthContext,
) -> ApiResult<()> {
    if !gateway_projects::attach_principal(&mut **tx, account, project, principal).await? {
        return Err(ApiError::NotFound("Active project key not found.".into()));
    }
    audit(
        tx, account, &auth.user.id, AuditEvent::GatewayProjectKeyAttached,
        "gateway_service_principal", principal.as_str(),
    ).await
}

pub async fn rotate_key(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, principal)): Path<(String, String)>,
    Json(input): Json<NewKeyInput>,
) -> ApiResult<(StatusCode, Json<IssuedKey>)> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    let principal = parse_id(&principal, "principal_id")?;
    let mut tx = state.pool.begin().await?;
    let issued = issue_key(
        &mut tx,
        &account,
        &project.id,
        principal,
        input.expires_in_days,
    )
    .await?;
    audit(
        &mut tx,
        &account,
        &auth.user.id,
        AuditEvent::ApiKeyCreated,
        "gateway_project_key",
        issued.id.as_str(),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(issued)))
}

pub async fn revoke_key(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, key)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    let key: GatewayProjectKeyId = parse_id(&key, "key_id")?;
    let mut tx = state.pool.begin().await?;
    gateway_projects::lock_project_key(&mut *tx, &account, &project.id).await?;
    if !gateway_projects::revoke_key(&mut *tx, &account, &project.id, &key).await? {
        return Err(ApiError::NotFound("project credential not found".into()));
    }
    audit(
        &mut tx,
        &account,
        &auth.user.id,
        AuditEvent::ApiKeyRevoked,
        "gateway_project_key",
        key.as_str(),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_principal(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Path((raw, principal)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let (account, project) = managed_project(&state, &auth, &query, &raw).await?;
    let principal: GatewayPrincipalId = parse_id(&principal, "principal_id")?;
    let mut tx = state.pool.begin().await?;
    gateway_projects::lock_project_key(&mut *tx, &account, &project.id).await?;
    if !gateway_projects::revoke_principal(&mut *tx, &account, &project.id, &principal).await? {
        return Err(ApiError::NotFound("service principal not found".into()));
    }
    gateway_projects::detach_principal(&mut *tx, &account, &project.id, &principal).await?;
    audit(
        &mut tx,
        &account,
        &auth.user.id,
        AuditEvent::ApiKeyRevoked,
        "gateway_service_principal",
        principal.as_str(),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn issue_key(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &AccountId,
    project: &GatewayProjectId,
    principal: GatewayPrincipalId,
    expires_in_days: Option<u16>,
) -> ApiResult<IssuedKey> {
    let days = expires_in_days.unwrap_or(90);
    if !(1..=365).contains(&days) {
        return Err(ApiError::BadRequest(
            "expires_in_days must be between 1 and 365".into(),
        ));
    }
    let mut random = [0; 32];
    OsRng.fill_bytes(&mut random);
    let token = format!("{PROJECT_KEY_PREFIX}{}", hex::encode(random));
    let key_prefix = token[..PROJECT_KEY_PREFIX.len() + 8].to_owned();
    let id = GatewayProjectKeyId::generate();
    let expires_at = Utc::now() + Duration::days(i64::from(days));
    if !gateway_projects::insert_key(
        &mut **tx,
        NewProjectKey {
            id: &id,
            account,
            project,
            principal: &principal,
            prefix: &key_prefix,
            hash: &sha256_hex(&token),
            expires_at,
        },
    )
    .await?
    {
        return Err(ApiError::NotFound("service principal not found".into()));
    }
    Ok(IssuedKey {
        id,
        principal_id: principal,
        key_prefix,
        expires_at,
        token,
    })
}
