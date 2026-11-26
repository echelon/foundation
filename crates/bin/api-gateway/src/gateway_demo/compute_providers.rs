use std::{collections::BTreeSet, sync::Arc};

use axum::{extract::{Query, State}, Extension, Json};
use db_queries::{gateway_requests::RequestScope, gateway_routing_policies as storage};
use enums::AuditEvent;
use ids::{AccountId, GatewayProjectId};
use serde::{Deserialize, Serialize};

use super::{
    access::ScopeQuery,
    catalog::{valid_identifier, Catalog},
    catalog_refresh::CatalogStore,
    projects::{self, OrganizationQuery},
    provider_access::managed_account,
    providers::{local_chat::compute::{EndpointConfig, TargetView}, ProviderRuntime},
    routing_policy::Policies,
    routing_settings::{current, managed_target},
};
use crate::{error::{ApiError, ApiResult}, middleware::AuthContext, state::AppState};

#[derive(Serialize)]
pub(super) struct Inventory {
    revision: i32,
    pools: Vec<Pool>,
}

#[derive(Serialize)]
struct Pool {
    binding: String,
    cell: String,
    models: Vec<String>,
    targets: Vec<TargetView>,
    order: Option<Vec<String>>,
}

fn models(catalog: &Catalog, account: &AccountId, binding: &str) -> Vec<String> {
    catalog.offers.iter()
        .filter(|offer| offer.adapter == super::catalog::Adapter::OpenAiCompatibleChat
            && offer.local_binding.as_deref() == Some(binding)
            && catalog.visible_to(offer, Some(account)))
        .map(|offer| offer.model.clone()).collect::<BTreeSet<_>>().into_iter().collect()
}

pub(super) async fn inventory(
    State(state): State<AppState>,
    Extension(store): Extension<Arc<CatalogStore>>,
    Extension(runtime): Extension<Option<Arc<ProviderRuntime>>>,
    auth: AuthContext,
    Query(query): Query<ScopeQuery>,
) -> ApiResult<Json<Inventory>> {
    let (account, project) = managed_target(&state, &auth, query).await?;
    let revision = current(&state.pool, &account, project.as_ref()).await?;
    let catalog = store.current();
    let pools = runtime.as_deref().into_iter().flat_map(ProviderRuntime::compute_stacks)
        .filter_map(|stack| {
            let models = models(&catalog, &account, &stack.name);
            (!models.is_empty() || (project.is_none() && stack.has_owned_target(&account))).then(|| Pool {
                binding: stack.name.clone(),
                cell: stack.cell.clone(),
                models,
                targets: stack.target_views(&account),
                order: revision.policy.compute_order.get(&stack.name).cloned(),
            })
        }).collect();
    Ok(Json(Inventory { revision: revision.revision, pools }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Update {
    expected_revision: i32,
    binding: String,
    order: Option<Vec<String>>,
}

pub(super) async fn update(
    State(state): State<AppState>,
    Extension(store): Extension<Arc<CatalogStore>>,
    Extension(runtime): Extension<Option<Arc<ProviderRuntime>>>,
    auth: AuthContext,
    Query(query): Query<ScopeQuery>,
    Json(input): Json<Update>,
) -> ApiResult<Json<i32>> {
    let (account, project) = managed_target(&state, &auth, query).await?;
    let project = project.ok_or_else(|| ApiError::BadRequest("select a project to set fallback".into()))?;
    let catalog = store.current();
    let stack = runtime.as_deref().and_then(|runtime| runtime.compute_stack(&input.binding))
        .filter(|stack| !models(&catalog, &account, &stack.name).is_empty())
        .ok_or_else(|| ApiError::NotFound("compute pool not found".into()))?;
    if let Some(order) = &input.order {
        let visible = stack.target_views(&account);
        let distinct: BTreeSet<_> = order.iter().collect();
        if order.is_empty() || order.len() > 4 || distinct.len() != order.len()
            || order.iter().any(|id| !visible.iter().any(|target| target.id == *id))
        {
            return Err(ApiError::BadRequest("select one to four distinct targets in this pool".into()));
        }
    }
    let mut tx = state.pool.begin().await?;
    storage::lock_for_update(&mut *tx, &account).await?;
    let mut prior = current(&mut *tx, &account, Some(&project)).await?;
    if prior.revision != input.expected_revision {
        return Err(ApiError::Conflict("routing policy changed; refresh compute fallback".into()));
    }
    if prior.policy.compute_order.get(&input.binding) == input.order.as_ref() {
        return Ok(Json(prior.revision));
    }
    match input.order {
        Some(order) => { prior.policy.compute_order.insert(input.binding, order); }
        None => { prior.policy.compute_order.remove(&input.binding); }
    }
    let revision = prior.revision.checked_add(1)
        .ok_or_else(|| ApiError::Internal("routing revision is out of range".into()))?;
    let policy = serde_json::to_value(&prior.policy)
        .map_err(|_| ApiError::Internal("could not encode routing policy".into()))?;
    storage::insert(&mut *tx, &account, Some(&project), revision, &policy, &auth.user.id).await?;
    projects::audit(&mut tx, &account, &auth.user.id, AuditEvent::GatewayRoutingPolicyChanged,
        "gateway_routing_policy", project.as_str()).await?;
    tx.commit().await?;
    Ok(Json(revision))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Plugin { Aws, GoogleCloud, Render, Custom }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Draft {
    plugin: Plugin,
    binding: String,
    cell: String,
    target: EndpointConfig,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComputePatch {
    pub version: u8,
    pub cell: String,
    pub binding: String,
    pub target: EndpointConfig,
}

pub(super) async fn preview(
    State(state): State<AppState>,
    Extension(store): Extension<Arc<CatalogStore>>,
    Extension(runtime): Extension<Option<Arc<ProviderRuntime>>>,
    auth: AuthContext,
    Query(query): Query<OrganizationQuery>,
    Json(mut draft): Json<Draft>,
) -> ApiResult<Json<ComputePatch>> {
    let account = managed_account(&state, &auth, &query).await?;
    if !valid_identifier(&draft.binding) || !valid_identifier(&draft.cell) {
        return Err(ApiError::BadRequest("enter valid binding and cell IDs".into()));
    }
    if let Some(cloud) = match draft.plugin {
        Plugin::Aws => Some("aws"), Plugin::GoogleCloud => Some("google_cloud"),
        Plugin::Render => Some("render"), Plugin::Custom => None,
    } {
        draft.target.location.cloud = cloud.to_owned();
    }
    draft.target.organization_id = Some(account.clone());
    let endpoint = draft.target.validate().map_err(|error| ApiError::BadRequest(error.to_string()))?;
    draft.target.endpoint = endpoint.to_string();
    if let Some(runtime) = runtime.as_deref() {
        let pool = runtime.compute_stack(&draft.binding);
        if runtime.has_chat_binding(&draft.binding) && pool.is_none() {
            return Err(ApiError::Conflict("choose a new binding name or an existing compute pool".into()));
        }
        if runtime.compute_stacks().any(|stack| stack.cell != draft.cell) {
            return Err(ApiError::BadRequest("use the compute registry's current cell ID".into()));
        }
        if let Some(pool) = pool {
            if !pool.has_owned_target(&account) && models(&store.current(), &account, &pool.name).is_empty() {
                return Err(ApiError::NotFound("compute pool not found".into()));
            }
            pool.review_addition(&draft.target)
                .map_err(|error| ApiError::BadRequest(error.to_string()))?;
        }
    }
    Ok(Json(ComputePatch { version: 1, cell: draft.cell, binding: draft.binding, target: draft.target }))
}

pub(in crate::gateway_demo) fn order<'a>(policies: &'a Policies, binding: &str) -> Option<&'a [String]> {
    policies.project.policy.compute_order.get(binding).map(Vec::as_slice)
}

pub(in crate::gateway_demo) async fn dispatch_policy(
    pool: &db::PgPool, request_id: &str,
) -> ApiResult<(AccountId, Policies)> {
    let (account, project): (AccountId, GatewayProjectId) = sqlx::query_as(
        "SELECT account_id, project_id FROM gateway_requests WHERE id = $1",
    ).bind(request_id).fetch_one(pool).await?;
    let policies = Policies::load(pool, RequestScope { account: &account, project: &project, client_ref: None }).await?;
    Ok((account, policies))
}
