
use db_queries::{
    gateway_requests::RequestScope,
};
use serde::{Deserialize, Serialize};

use super::{
    availability::{AvailabilityInput, RouteAvailability},
    catalog::{digest, Catalog},
    providers::ProviderRuntime,
    quote::{Quote, QuoteInput, Reason},
    requests,
    routing_policy::Policies,
};
use crate::{
    error::{ApiError, ApiResult},
};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdvisorSource {
    Request,
    Policy,
}

pub(super) struct AdvisorSelection {
    pub source: AdvisorSource,
}

impl AdvisorSelection {
    pub(super) fn validate_policy(&self, policies: &Policies) -> ApiResult<()> {
        if self.source == AdvisorSource::Policy && !policies.advisor_enabled() {
            return Err(ApiError::BadRequest(
                "supply a model or enable Advisor routing in policy".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AdvisorDecision {
    pub model: String,
    #[schema(inline)]
    pub source: AdvisorSource,
    pub used_advisor: bool,
}

pub(super) struct AdvisorRoute<'a, 'scope> {
    pub pool: &'a db::PgPool,
    pub scope: RequestScope<'scope>,
    pub catalog: &'a Catalog,
    pub providers: Option<&'a ProviderRuntime>,
    pub selection: &'a AdvisorSelection,
    pub input: &'a QuoteInput,
}

pub(super) async fn select(route: AdvisorRoute<'_, '_>) -> ApiResult<Option<AdvisorDecision>> {
    let policies = Policies::load(route.pool, route.scope).await?;
    route.selection.validate_policy(&policies)?;
    let availability = RouteAvailability::for_pool(route.pool, AvailabilityInput {
        scope: route.scope,
        catalog: route.catalog,
        providers: route.providers,
        quote: route.input,
        policies: &policies,
    }).await?;
    let quote = requests::media_quote(
        route.catalog, route.input.clone(), route.scope, &policies, route.providers, &availability,
    )?;
    let Some(model) = quote.selected_offer_id.as_ref().and_then(|id| {
        quote.candidates.iter().find(|candidate| &candidate.offer_id == id)?.model.clone()
    }) else {
        return Ok(None);
    };
    Ok(Some(AdvisorDecision { model, source: route.selection.source, used_advisor: false }))
}

pub(super) fn apply(quote: &mut Quote, decision: AdvisorDecision) -> ApiResult<()> {
    let position = quote.candidates.iter().position(|candidate| {
        candidate.reason == Reason::Eligible && candidate.model.as_deref() == Some(decision.model.as_str())
    }).ok_or_else(|| ApiError::Conflict("the advised model no longer meets the routing requirements".into()))?;

    quote.candidates[..=position].rotate_right(1);
    quote.selected_offer_id = Some(quote.candidates[0].offer_id.clone());
    quote.id = format!("quo_{}", digest(format!("advisor-v1:{}:{}", quote.id, decision.model)));
    quote.advisor = Some(decision);
    Ok(())
}
