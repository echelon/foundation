use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use db_queries::{
    gateway_provider_credentials::{self, Effective},
    gateway_requests::{queue_cooldowns, RequestScope},
    model_reviews::{self, UsageFilter},
};

use crate::{error::ApiResult, model_reviews::MIN_QUALITY_RATINGS};

use super::{
    catalog::{alternate_model_id, public_model_id, Catalog},
    providers::ProviderRuntime,
    quote::{QuoteInput, Reason, Workload},
    routing_policy::Policies,
};

#[derive(Default)]
pub(super) struct RouteAvailability {
    queue: BTreeMap<String, DateTime<Utc>>,
    model_exclusions: BTreeMap<String, Reason>,
    compute_unavailable: BTreeSet<String>,
    credentials: BTreeMap<String, Option<Effective>>,
}

pub(super) struct AvailabilityInput<'scope, 'a> {
    pub scope: RequestScope<'scope>,
    pub catalog: &'a Catalog,
    pub providers: Option<&'a ProviderRuntime>,
    pub quote: &'a QuoteInput,
    pub policies: &'a Policies,
}

impl RouteAvailability {
    pub(super) async fn for_pool(
        pool: &db::PgPool,
        input: AvailabilityInput<'_, '_>,
    ) -> ApiResult<Self> {
        if input.providers.is_none() && !input.quote.is_auto() {
            return Ok(Self::default());
        }
        let mut connection = pool.acquire().await?;
        Self::load(&mut connection, input).await
    }

    pub(super) async fn load(
        connection: &mut sqlx::PgConnection,
        input: AvailabilityInput<'_, '_>,
    ) -> ApiResult<Self> {
        let AvailabilityInput { scope, catalog, providers, quote, policies } = input;
        quote.validate_model_selection()?;
        let offers = catalog.offers.iter().filter(|offer| {
            policies.shares_connection(catalog, &offer.connection_id(), scope)
                && quote.considers(offer)
                && quote.provider.as_ref().is_none_or(|provider| &offer.provider == provider)
        });
        let model_exclusions = match policies.resolution(quote).filter(|_| quote.is_auto()) {
            Some(resolution) if resolution.min_quality.is_some() || resolution.min_usage_accounts.is_some() => {
                let models = offers
                    .clone()
                    .filter(|offer| policies.allows_offer(catalog, offer, scope))
                    .map(|offer| public_model_id(&offer.model).to_owned())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                model_exclusions(connection, models, resolution).await?
            }
            _ => BTreeMap::new(),
        };
        let mut result = Self { model_exclusions, ..Self::default() };
        let Some(providers) = providers else {
            return Ok(result);
        };
        let tenant_offers: Vec<_> = offers.clone().filter_map(|offer| {
            let name = offer.local_binding.as_deref()?;
            let (adapter, digest) = providers.tenant_binding(name)?;
            (adapter == offer.adapter).then_some((offer, name, adapter.as_str(), digest))
        }).collect();
        if !tenant_offers.is_empty() {
            let bindings: Vec<_> = tenant_offers.iter()
                .map(|(_, name, adapter, _)| ((*adapter).to_owned(), (*name).to_owned()))
                .collect::<BTreeSet<_>>().into_iter().collect();
            let effective = gateway_provider_credentials::effective(&mut *connection, scope, &bindings).await?;
            result.credentials = tenant_offers.into_iter().map(|(offer, name, adapter, digest)| {
                let pin = effective.iter().find(|key| key.binding == name && key.adapter == adapter
                    && key.usable && key.binding_digest == digest);
                (offer.id.clone(), pin.cloned())
            }).collect();
        }
        result.compute_unavailable = offers.clone()
            .filter(|offer| !providers.has_compute_route(offer, scope.account, policies))
            .map(|offer| offer.id.clone()).collect();
        if matches!(quote.workload, Workload::Text { .. }) {
            return Ok(result);
        }
        let queued: Vec<_> = offers
            .filter_map(|offer| providers.queue_offer_binding(offer).map(|binding| (offer, binding)))
            .collect();
        if !queued.is_empty() {
            let active = queue_cooldowns::active(&mut *connection, scope).await?;
            result.queue = queued
                .into_iter()
                .filter_map(|(offer, binding)| {
                    let cooldown = active.iter().find(|entry| {
                        entry.binding_digest == binding
                            && offer.upstream_model.as_deref() == Some(entry.upstream_model.as_str())
                    })?;
                    Some((offer.id.clone(), cooldown.available_after))
                })
                .collect();
        }
        Ok(result)
    }

    pub(super) fn model_exclusion(&self, model: &str) -> Option<Reason> {
        self.model_exclusions.get(model).copied()
    }

    pub(super) fn compute_unavailable(&self, offer_id: &str) -> bool {
        self.compute_unavailable.contains(offer_id)
    }

    pub(super) fn credential_unavailable(&self, offer: &str) -> bool {
        self.credentials.get(offer).is_some_and(Option::is_none)
    }

    pub(super) fn credential(&self, offer: &str) -> Option<&Effective> {
        self.credentials.get(offer).and_then(Option::as_ref)
    }

    pub(super) fn available_after(
        &self,
        offer_id: &str,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        self.queue
            .get(offer_id)
            .copied()
            .filter(|deadline| *deadline > now)
    }
}

async fn model_exclusions(
    connection: &mut sqlx::PgConnection,
    mut models: Vec<String>,
    resolution: super::routing_policy::Resolution,
) -> ApiResult<BTreeMap<String, Reason>> {
    let mut exclusions = BTreeMap::new();
    if let Some(minimum) = resolution.min_quality {
        let ratings: BTreeMap<_, _> = model_reviews::model_ratings(&mut *connection, &models)
            .await?
            .into_iter()
            .map(|row| (row.model, (row.rating_count, row.rating_total)))
            .collect();
        models.retain(|model| {
            let (count, total) = ratings.get(model).copied().unwrap_or_default();
            let reason = if count < MIN_QUALITY_RATINGS {
                Reason::InsufficientModelRatings
            } else if i128::from(total) < i128::from(count) * i128::from(minimum.value) {
                Reason::BelowModelQuality
            } else {
                return true;
            };
            exclusions.insert(model.clone(), reason);
            false
        });
    }
    if let Some(minimum) = resolution.min_usage_accounts.filter(|_| !models.is_empty()) {
        let aliases: Vec<_> = models.iter()
            .map(|model| alternate_model_id(model).unwrap_or(model).to_owned())
            .collect();
        let qualified: BTreeSet<_> = model_reviews::used_models(
            connection,
            UsageFilter { models: &models, aliases: &aliases, minimum_accounts: i64::from(minimum.value) },
        ).await?.into_iter().collect();
        exclusions.extend(
            models.into_iter().filter(|model| !qualified.contains(model)).map(|model| (model, Reason::BelowModelUsage))
        );
    }
    Ok(exclusions)
}
