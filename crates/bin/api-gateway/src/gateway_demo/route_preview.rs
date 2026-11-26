use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use db_queries::gateway_requests::{read_snapshot, RequestScope};
use serde::{Deserialize, Serialize};

use super::{
    access::{self, GatewayCaller, Permission, ScopeQuery},
    availability::{AvailabilityInput, RouteAvailability},
    catalog::Provider,
    catalog_refresh::CatalogStore,
    handlers::parse_request_id,
    providers::ProviderRuntime,
    quote::{Quote, QuoteProfile, QuoteTarget},
    requests, route_performance,
    routing_policy::Policies,
};
use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RoutePreviewOptions)]
pub(super) struct Options {
    /// Replace the saved per-request cost cap for this preview.
    max_total_usd_nanos: Option<i64>,
    /// Replace the saved provider order. An empty list uses estimated cost.
    preferred_providers: Option<Vec<Provider>>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
enum UnavailableReason {
    PinnedOfferUnavailable,
    WorkloadUnavailable,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(super) struct RoutePreview {
    observed_at: DateTime<Utc>,
    recorded: Quote,
    current: Option<Quote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unavailable_reason: Option<UnavailableReason>,
}

#[utoipa::path(
    post, path = "/v1/requests/{id}/route-preview", operation_id = "previewGatewayRequestRoute", tag = "native",
    summary = "Compare a saved route with a current quote",
    description = "Requotes the saved workload under current policy in a read-only transaction scoped to the original client.",
    security(("project_api_key" = []), ("session_cookie" = []), ("api_key" = [])),
    params(("id" = String, Path, description = "Durable request ID."), ScopeQuery),
    request_body = Options,
    responses(
        (status = 200, description = "Recorded route and read-only current preview.", body = RoutePreview),
        (status = 404, description = "Saved request is outside scope or unavailable.", body = crate::error::GatewayError),
        (status = "default", response = super::RequestErrorResponse),
    ),
)]
pub(super) async fn preview(
    State(state): State<AppState>,
    Extension(catalog): Extension<Arc<CatalogStore>>,
    Extension(providers): Extension<Option<Arc<ProviderRuntime>>>,
    caller: GatewayCaller,
    Query(query): Query<ScopeQuery>,
    Path(raw): Path<String>,
    Json(options): Json<Options>,
) -> ApiResult<Json<RoutePreview>> {
    let access = access::resolve(&state, caller, query, Permission::ReadRequests).await?;
    let id = parse_request_id(&raw)?;

    let mut tx = state.pool.begin().await?;
    read_snapshot(&mut *tx).await?;
    let request = requests::get(&mut *tx, access.scope(), &id).await?;
    let scope = RequestScope {
        client_ref: request.client_ref.as_deref(),
        ..access.scope()
    };
    let recorded = request.quote;
    let profile = recorded.profile.unwrap_or_default();
    let mut input = recorded.input.clone();
    if let Some(cap) = options.max_total_usd_nanos {
        input.max_total_usd_nanos = Some(cap);
    }
    if let Some(order) = options.preferred_providers {
        input.preferred_providers = Some(order);
    }

    let catalog = catalog.current();
    let policies = Policies::load(&mut *tx, scope).await?;
    let availability =
        RouteAvailability::load(&mut *tx, AvailabilityInput {
            scope, catalog: &catalog, providers: providers.as_deref(), quote: &input, policies: &policies,
        }).await?;
    let current = match profile {
        QuoteProfile::QueueMedia => requests::media_quote(
            &catalog,
            input,
            scope,
            &policies,
            providers.as_deref(),
            &availability,
        ),
        QuoteProfile::Chat => {
            catalog.scoped_quote(input, scope, &policies, &availability, QuoteTarget::Chat)
        }
        QuoteProfile::Native => {
            let target = recorded
                .pinned_offer_id
                .as_deref()
                .map_or(QuoteTarget::Any, QuoteTarget::Offer);
            catalog.scoped_quote(input, scope, &policies, &availability, target)
        }
    };
    let (current, unavailable_reason) = match current {
        Ok(quote) => {
            let mut quote =
                route_performance::rank(&mut *tx, scope, &catalog, providers.as_deref(), quote)
                    .await?;
            quote.profile = Some(profile);
            quote.chat_options = recorded.chat_options;
            (Some(quote.into_preview()), None)
        }
        Err(ApiError::NotFound(_)) => (
            None,
            Some(if recorded.pinned_offer_id.is_some() {
                UnavailableReason::PinnedOfferUnavailable
            } else {
                UnavailableReason::WorkloadUnavailable
            }),
        ),
        Err(error) => return Err(error),
    };
    tx.commit().await?;
    let preview = RoutePreview {
        observed_at: Utc::now(),
        recorded,
        current,
        unavailable_reason,
    };
    Ok(Json(preview))
}
