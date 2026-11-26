use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use db_queries::{
    gateway_requests::{
        active_requests::{self, ActiveRequests},
        monthly_budget::{self, MonthlyBudget},
        RequestScope,
    },
    gateway_routing_policies as storage,
};
use enums::{AuditEvent, MemberRole};
use ids::{AccountId, GatewayProjectId};
use serde::{Deserialize, Serialize};

use super::{
    access::{self, Access, GatewayCaller, Permission, ScopeQuery},
    availability::{AvailabilityInput, RouteAvailability},
    catalog::{public_model_id, ModelQuote},
    catalog_refresh::CatalogStore,
    projects::{self, OrganizationQuery},
    providers::ProviderRuntime,
    quote::{Quote, QuoteInput, QuoteTarget},
    quote_seals::QuoteSeals,
    route_performance,
    routing_policy::{Policies, Policy, Revision},
};
use crate::{
    authz::require_account_role,
    error::{ApiError, ApiResult},
    middleware::AuthContext,
    state::AppState,
};

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct QuoteScopeQuery {
    /// Account ID for session callers. A bearer key already carries its account.
    account_id: Option<String>,
    /// Project ID. Sessions without this value use their account's default project.
    project_id: Option<String>,
    /// Client reference for this quote. Use the same reference when submitting.
    client_ref: Option<String>,
    /// Exact gateway offer to quote for this model.
    pinned_offer_id: Option<String>,
}

impl QuoteScopeQuery {
    async fn resolve(&self, state: &AppState, caller: GatewayCaller) -> ApiResult<Access> {
        let scope = ScopeQuery {
            account_id: self.account_id.clone(),
            project_id: self.project_id.clone(),
        };
        access::resolve(state, caller, scope, Permission::SubmitRequests).await
    }
}

#[utoipa::path(
    post, path = "/v1/quotes/scoped", operation_id = "quoteProjectRequest", tag = "native",
    summary = "Quote within a project",
    description = "Applies project and organization routing policy. A project key requires requests:submit. The signed quote expires after five minutes or sooner if a candidate rate expires. No provider work is started.",
    security(("project_api_key" = []), ("session_cookie" = []), ("api_key" = [])),
    params(QuoteScopeQuery),
    request_body = QuoteInput,
    responses(
        (status = 200, description = "Project quote and routing evidence.", body = Quote),
        (status = "default", response = super::RequestErrorResponse),
    ),
)]
pub async fn scoped_quote(
    State(state): State<AppState>,
    Extension(seals): Extension<Arc<QuoteSeals>>,
    Extension(catalog): Extension<Arc<CatalogStore>>,
    Extension(providers): Extension<Option<Arc<ProviderRuntime>>>,
    caller: GatewayCaller,
    Query(query): Query<QuoteScopeQuery>,
    Json(input): Json<QuoteInput>,
) -> ApiResult<Json<Quote>> {
    let access = query.resolve(&state, caller).await?;
    let scope = access.for_client(query.client_ref.as_deref())?;
    let policies = Policies::load(&state.pool, scope).await?;
    let catalog = catalog.current();
    let availability = RouteAvailability::for_pool(&state.pool, AvailabilityInput {
        scope, catalog: &catalog, providers: providers.as_deref(), quote: &input, policies: &policies,
    }).await?;
    let target = query.pinned_offer_id.as_deref().map_or(QuoteTarget::Any, QuoteTarget::Offer);
    let quote = catalog.scoped_quote(input, scope, &policies, &availability, target)?;
    let mut quote = route_performance::rank(&state.pool, scope, &catalog, providers.as_deref(), quote).await?;
    seals.seal(&mut quote, scope)?;
    route_performance::apply_visibility(&access, &mut quote);
    Ok(Json(quote))
}

pub async fn get(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ScopeQuery>,
) -> ApiResult<Json<PolicyView>> {
    let (account, project) = managed_target(&state, &auth, query).await?;
    let mut conn = state.pool.acquire().await?;
    let revision = current(&mut *conn, &account, project.as_ref()).await?;
    Ok(Json(view(&mut conn, &account, project.as_ref(), revision).await?))
}

#[derive(Serialize)]
pub struct PolicyView {
    #[serde(flatten)]
    revision: Revision,
    #[serde(skip_serializing_if = "Option::is_none")]
    monthly_budget: Option<MonthlyBudget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_requests: Option<ActiveRequests>,
}

async fn view(
    conn: &mut sqlx::PgConnection,
    account: &AccountId,
    project: Option<&GatewayProjectId>,
    revision: Revision,
) -> ApiResult<PolicyView> {
    let monthly_budget = match revision.policy.monthly_estimated_budget_usd_nanos {
        Some(limit) => Some(monthly_budget::usage(&mut *conn, account, project, None, Some(limit)).await?),
        None => None,
    };
    let active_requests = if revision.policy.active_request_limits.is_empty() {
        None
    } else {
        Some(active_requests::usage(&mut *conn, account, project).await?)
    };
    Ok(PolicyView { revision, monthly_budget, active_requests })
}

#[utoipa::path(
    post, path = "/v1/catalog/quotes/scoped", operation_id = "quoteProjectModel", tag = "catalog",
    summary = "Quote a named model within a project",
    description = "Applies project policy and connection availability. A project key requires requests:submit. Use /v1/quotes/scoped for automatic model selection or pinned offers.",
    security(("project_api_key" = []), ("session_cookie" = []), ("api_key" = [])),
    params(ScopeQuery, ("client_ref" = Option<String>, Query, description = "Use the same client reference when submitting.")),
    request_body = QuoteInput,
    responses(
        (status = 200, description = "Signed model quote without provider and offer IDs on candidates.", body = ModelQuote),
        (status = "default", response = super::RequestErrorResponse),
    ),
)]
pub(super) async fn scoped_model_quote(
    State(state): State<AppState>,
    Extension(seals): Extension<Arc<QuoteSeals>>,
    Extension(catalog): Extension<Arc<CatalogStore>>,
    Extension(providers): Extension<Option<Arc<ProviderRuntime>>>,
    caller: GatewayCaller,
    Query(query): Query<QuoteScopeQuery>,
    Json(mut input): Json<QuoteInput>,
) -> ApiResult<Json<ModelQuote>> {
    if input.is_auto() {
        return Err(ApiError::BadRequest("use /v1/quotes/scoped for automatic model selection".into()));
    }
    if query.pinned_offer_id.is_some() {
        return Err(ApiError::BadRequest("offer pins require /v1/quotes/scoped".into()));
    }
    input.provider = None;
    input.model = public_model_id(&input.model).to_owned();
    let access = query.resolve(&state, caller).await?;
    let scope = access.for_client(query.client_ref.as_deref())?;
    let policies = Policies::load(&state.pool, scope).await?;
    let catalog = catalog.current();
    let availability = RouteAvailability::for_pool(&state.pool, AvailabilityInput {
        scope, catalog: &catalog, providers: providers.as_deref(), quote: &input, policies: &policies,
    }).await?;
    let quote = catalog.scoped_quote(input, scope, &policies, &availability, QuoteTarget::Any)?;
    let mut quote = route_performance::rank(&state.pool, scope, &catalog, providers.as_deref(), quote).await?;
    seals.seal(&mut quote, scope)?;
    route_performance::apply_visibility(&access, &mut quote);
    Ok(Json(catalog.model_quote(quote)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyUpdate {
    expected_revision: i32,
    policy: Policy,
}

pub async fn update(
    State(state): State<AppState>,
    Extension(catalog): Extension<Arc<CatalogStore>>,
    auth: AuthContext,
    Query(query): Query<ScopeQuery>,
    Json(mut update): Json<PolicyUpdate>,
) -> ApiResult<Json<PolicyView>> {
    let (account, project) = managed_target(&state, &auth, query).await?;
    update.policy.validate(&catalog.current())?;
    let mut tx = state.pool.begin().await?;
    storage::lock_for_update(&mut *tx, &account).await?;
    let prior = current(&mut *tx, &account, project.as_ref()).await?;
    if prior.revision != update.expected_revision {
        return Err(ApiError::Conflict(
            "routing policy changed; reload its current revision".into(),
        ));
    }
    update.policy.connection_projects.clone_from(&prior.policy.connection_projects);
    update.policy.excluded_connections.clone_from(&prior.policy.excluded_connections);
    update.policy.compute_order.clone_from(&prior.policy.compute_order);
    let next = prior.next(update.policy)?;
    let encoded = serde_json::to_value(&next.policy)
        .map_err(|_| ApiError::Internal("could not encode routing policy".into()))?;
    storage::insert(
        &mut *tx,
        &account,
        project.as_ref(),
        next.revision,
        &encoded,
        &auth.user.id,
    )
    .await?;
    projects::audit(
        &mut tx,
        &account,
        &auth.user.id,
        AuditEvent::GatewayRoutingPolicyChanged,
        "gateway_routing_policy",
        project
            .as_ref()
            .map_or(account.as_str(), GatewayProjectId::as_str),
    )
    .await?;
    let view = view(&mut *tx, &account, project.as_ref(), next).await?;
    tx.commit().await?;
    Ok(Json(view))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Layer {
    Organization,
    Project,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftPreview {
    layer: Layer,
    policy: Policy,
    input: QuoteInput,
}

pub async fn preview(
    State(state): State<AppState>,
    Extension(catalog): Extension<Arc<CatalogStore>>,
    Extension(providers): Extension<Option<Arc<ProviderRuntime>>>,
    auth: AuthContext,
    Query(query): Query<ScopeQuery>,
    Json(mut draft): Json<DraftPreview>,
) -> ApiResult<Json<Quote>> {
    let (account, project) = managed_target(&state, &auth, query).await?;
    let project = project.ok_or_else(|| ApiError::BadRequest("project_id is required".into()))?;
    let catalog = catalog.current();
    draft.policy.validate(&catalog)?;
    let scope = RequestScope {
        account: &account,
        project: &project,
        client_ref: None,
    };
    let mut policies = Policies::load(&state.pool, scope).await?;
    let proposed = match draft.layer {
        Layer::Organization => &mut policies.organization,
        Layer::Project => &mut policies.project,
    };
    draft.policy.connection_projects.clone_from(&proposed.policy.connection_projects);
    draft.policy.excluded_connections.clone_from(&proposed.policy.excluded_connections);
    draft.policy.compute_order.clone_from(&proposed.policy.compute_order);
    *proposed = proposed.next(draft.policy)?;
    let availability = RouteAvailability::for_pool(&state.pool, AvailabilityInput {
        scope, catalog: &catalog, providers: providers.as_deref(), quote: &draft.input, policies: &policies,
    }).await?;
    let quote = catalog.scoped_quote(draft.input, scope, &policies, &availability, QuoteTarget::Any)?;
    let quote = route_performance::rank(&state.pool, scope, &catalog, providers.as_deref(), quote).await?;
    Ok(Json(quote.into_preview()))
}

pub(super) async fn current<'e>(
    exec: impl sqlx::PgExecutor<'e>,
    account: &AccountId,
    project: Option<&GatewayProjectId>,
) -> ApiResult<Revision> {
    storage::current(exec, account, project)
        .await?
        .into_iter()
        .find(|row| row.project_id.is_some() == project.is_some())
        .map(Revision::from_row)
        .transpose()
        .map(Option::unwrap_or_default)
}

pub(super) async fn managed_target(
    state: &AppState,
    auth: &AuthContext,
    query: ScopeQuery,
) -> ApiResult<(AccountId, Option<GatewayProjectId>)> {
    let account = query
        .account_id
        .ok_or_else(|| ApiError::BadRequest("account_id is required".into()))?;
    let organization = OrganizationQuery::new(account);
    if let Some(project_id) = query.project_id {
        let (account, project) =
            projects::managed_project(state, auth, &organization, &project_id).await?;
        return Ok((account, Some(project.id)));
    }
    let account = organization.account()?;
    require_account_role(&state.pool, &auth.user.id, &account, MemberRole::ADMINS).await?;
    Ok((account, None))
}
