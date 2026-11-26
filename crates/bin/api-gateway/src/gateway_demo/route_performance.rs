use std::cmp::Ordering;

use db_queries::gateway_requests::{route_performance, RequestScope};
use serde::{Deserialize, Serialize};

use super::{
    access::Access,
    catalog::{digest, Catalog},
    providers::ProviderRuntime,
    quote::{Candidate, Quote, Reason},
    routing_policy::RouteRanking,
};
use crate::error::{ApiError, ApiResult};

const WINDOW_DAYS: i32 = 7;
const MINIMUM_SAMPLES: i64 = 20;
const MAXIMUM_SAMPLES: i64 = 100;

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(super) struct OfferPerformance {
    pub samples: i64,
    pub succeeded: i64,
    pub unconfirmed: i64,
    pub median_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(super) struct CostEvidence {
    /// Comparable requests with a supplier settlement, including explicit zero amounts.
    pub settled: i64,
    /// Sum of the latest posted amounts in decimal nano-USD.
    pub settled_usd_nanos: Option<String>,
}

impl OfferPerformance {
    fn fully_settled(&self) -> bool {
        self.samples >= MINIMUM_SAMPLES
            && self
                .cost
                .as_ref()
                .is_some_and(|cost| cost.settled == self.samples)
    }

    fn cost_per_completion(&self) -> Option<(u128, u128)> {
        if self.succeeded <= 0 {
            return None;
        }
        let total = self
            .cost
            .as_ref()?
            .settled_usd_nanos
            .as_ref()?
            .parse()
            .ok()?;
        Some((total, self.succeeded as u128))
    }
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(super) struct PerformanceWindow {
    pub window_days: i32,
    pub minimum_samples: i64,
    pub maximum_samples: i64,
    pub applied: bool,
}

/// Attach recent outcomes and apply the configured ranking to the highest-priority eligible group.
pub(super) async fn rank<'e>(
    exec: impl sqlx::PgExecutor<'e>,
    scope: RequestScope<'_>,
    catalog: &Catalog,
    providers: Option<&ProviderRuntime>,
    mut quote: Quote,
) -> ApiResult<Quote> {
    let strategy = quote
        .routing
        .as_ref()
        .and_then(|policy| policy.route_ranking.as_ref())
        .map(|setting| setting.value)
        .unwrap_or(RouteRanking::Price);
    if strategy == RouteRanking::Price
        || quote.selected_offer_id.is_none()
        || quote.pinned_offer_id.is_some()
    {
        return Ok(quote);
    }
    let include_cost = strategy == RouteRanking::CostPerCompletion;

    let priority = quote
        .candidates
        .first()
        .and_then(|candidate| candidate.preference_rank);
    let group = quote
        .candidates
        .iter()
        .take_while(|candidate| {
            candidate.reason == Reason::Eligible && candidate.preference_rank == priority
        })
        .count();
    let Some(providers) = providers else {
        return Ok(quote);
    };
    let targets: Vec<_> = quote
        .candidates
        .iter()
        .filter(|candidate| candidate.reason == Reason::Eligible)
        .filter_map(|candidate| {
            let offer = catalog
                .offers
                .iter()
                .find(|offer| offer.id == candidate.offer_id)?;
            let binding = providers.queue_offer_binding(offer)?;
            Some(serde_json::json!({
                "id": candidate.offer_id,
                "binding_digest": binding,
                "upstream_model": offer.upstream_model,
                "applied_aspect_ratio": candidate.applied_aspect_ratio,
                "output_size": candidate.output_size,
                "estimated_usd_nanos": candidate.estimated_usd_nanos,
            }))
        })
        .collect();
    if targets.is_empty() {
        return Ok(quote);
    }
    let workload = serde_json::to_value(&quote.input.workload)
        .map_err(|_| ApiError::Internal("could not encode performance workload".into()))?;
    let observed = route_performance::recent_outcomes(
        exec,
        scope,
        &workload,
        &serde_json::Value::Array(targets),
        WINDOW_DAYS,
        MAXIMUM_SAMPLES,
        include_cost,
    )
    .await?;
    for row in observed {
        if let Some(candidate) = quote
            .candidates
            .iter_mut()
            .find(|candidate| candidate.offer_id == row.offer_id)
        {
            candidate.performance = Some(OfferPerformance {
                samples: row.samples,
                succeeded: row.succeeded,
                unconfirmed: row.unconfirmed,
                median_ms: row.median_ms,
                cost: include_cost.then(|| CostEvidence {
                    settled: row.settled_samples,
                    settled_usd_nanos: row.settled_usd_nanos,
                }),
            });
        }
    }

    let eligible_group = &mut quote.candidates[..group];
    let applied = group > 1
        && if include_cost {
            ready_for_cost_ranking(eligible_group)
        } else {
            eligible_group.iter().all(|candidate| {
                candidate
                    .performance
                    .as_ref()
                    .is_some_and(|evidence| evidence.samples >= MINIMUM_SAMPLES)
            })
        };
    if applied {
        let prior: Vec<_> = eligible_group.iter().map(|c| c.offer_id.clone()).collect();
        // Stable sorting keeps estimated price and offer ID as the final tie breakers.
        eligible_group.sort_by(|a, b| match (&a.performance, &b.performance) {
            (Some(left), Some(right)) if include_cost => {
                compare_cost(left, right).then_with(|| compare_performance(left, right))
            }
            (Some(left), Some(right)) => compare_performance(left, right),
            _ => Ordering::Equal,
        });
        let ranked: Vec<_> = eligible_group.iter().map(|c| c.offer_id.as_str()).collect();
        if ranked != prior {
            // Bind changes to route order, while allowing fresh measurements of an unchanged order.
            let identity = serde_json::to_vec(&(&quote.id, scope.client_ref, &ranked))
                .map_err(|_| ApiError::Internal("could not encode observed route order".into()))?;
            quote.id = format!("quo_{}", digest(identity));
        }
        quote.selected_offer_id = eligible_group.first().map(|c| c.offer_id.clone());
    }
    quote.performance_window = Some(PerformanceWindow {
        window_days: WINDOW_DAYS,
        minimum_samples: MINIMUM_SAMPLES,
        maximum_samples: MAXIMUM_SAMPLES,
        applied,
    });
    Ok(quote)
}

/// Historical measurements require the same permission as other request evidence.
pub(super) fn apply_visibility(access: &Access, quote: &mut Quote) {
    if !access.can_read_requests {
        for candidate in &mut quote.candidates {
            candidate.performance = None;
        }
        quote.performance_window = None;
    }
}

fn ready_for_cost_ranking(candidates: &[Candidate]) -> bool {
    let first = &candidates[0];
    candidates.iter().all(|candidate| {
        candidate.applied_aspect_ratio == first.applied_aspect_ratio
            && candidate.output_size == first.output_size
            && candidate
                .performance
                .as_ref()
                .is_some_and(OfferPerformance::fully_settled)
    }) && candidates.iter().any(|candidate| {
        candidate
            .performance
            .as_ref()
            .is_some_and(|evidence| evidence.succeeded > 0)
    })
}

fn compare_cost(left: &OfferPerformance, right: &OfferPerformance) -> Ordering {
    match (left.cost_per_completion(), right.cost_per_completion()) {
        // Compare exact fractions. Each total covers at most 100 nonnegative i64 amounts.
        (Some((left_total, left_successes)), Some((right_total, right_successes))) => {
            (left_total * right_successes).cmp(&(right_total * left_successes))
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_performance(left: &OfferPerformance, right: &OfferPerformance) -> Ordering {
    (right.succeeded * left.samples)
        .cmp(&(left.succeeded * right.samples))
        .then_with(|| {
            left.median_ms
                .unwrap_or(i64::MAX)
                .cmp(&right.median_ms.unwrap_or(i64::MAX))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_completion_fraction_then_successful_latency() {
        let slow = OfferPerformance {
            samples: 40,
            succeeded: 30,
            unconfirmed: 5,
            median_ms: Some(1500),
            cost: None,
        };
        let fast = OfferPerformance {
            samples: 20,
            succeeded: 15,
            unconfirmed: 2,
            median_ms: Some(750),
            cost: None,
        };
        let less_reliable = OfferPerformance {
            succeeded: 14,
            median_ms: Some(250),
            ..fast.clone()
        };
        assert_eq!(compare_performance(&fast, &slow), Ordering::Less);
        assert_eq!(compare_performance(&slow, &less_reliable), Ordering::Less);
        assert_eq!(compare_performance(&fast, &fast), Ordering::Equal);
    }

    #[test]
    fn compares_exact_settled_costs_and_keeps_zero_cost_distinct_from_zero_success() {
        let cost = |total: &str, succeeded| OfferPerformance {
            samples: 20,
            succeeded,
            unconfirmed: 0,
            median_ms: None,
            cost: Some(CostEvidence {
                settled: 20,
                settled_usd_nanos: Some(total.into()),
            }),
        };
        let first = cost("9007199254740995", 3);
        let mut second = cost("9007199254740994", 3);
        let no_completions = cost("0", 0);
        let fully_credited = cost("0", 20);
        assert_eq!(compare_cost(&second, &first), Ordering::Less);
        assert_eq!(compare_cost(&second, &no_completions), Ordering::Less);
        assert_eq!(compare_cost(&fully_credited, &second), Ordering::Less);
        assert_eq!(
            compare_cost(&second, &cost("18014398509481988", 6)),
            Ordering::Equal
        );
        assert!(second.fully_settled());
        second.cost.as_mut().unwrap().settled = 19;
        assert!(!second.fully_settled());
    }
}
