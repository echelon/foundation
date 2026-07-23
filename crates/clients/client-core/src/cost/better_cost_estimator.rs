use crate::cost::cost_breakdown::CostBreakdown;

/// The richer pricing interface (successor to
/// [`crate::cost::cost_calculator::CostCalculator`]): instead of one opaque
/// number, a request prices itself as a BREAKDOWN — an exact-or-estimated
/// total, the per-line components behind it, and the model's full rate card +
/// formula (structured enough for a pricing table or an interactive frontend
/// calculator). One trait covers the pricing shapes of LLMs (per-token
/// in/out), image models (per-image, possibly tiered), video models
/// (per-second by resolution, or token-priced), and music (per-song).
/// Implemented on each `*Request` next to its endpoint, in `cost.rs`.
pub trait BetterCostEstimator {
    fn estimate_cost(&self) -> CostBreakdown;
}

#[cfg(test)]
mod tests {
    use money::MicroUsd;

    use super::*;
    use crate::cost::cost_amount::CostAmount;
    use crate::cost::cost_certainty::CostCertainty;
    use crate::cost::cost_component::CostComponent;
    use crate::cost::cost_formula::CostFormula;
    use crate::cost::cost_quantity::CostQuantity;
    use crate::cost::cost_unit::CostUnit;
    use crate::cost::rate_row::RateRow;

    /// A miniature "video model": seconds × per-second rate — exercises the
    /// whole interface the way a real binding does.
    struct FakeVideoRequest {
        seconds: f64,
    }

    impl BetterCostEstimator for FakeVideoRequest {
        fn estimate_cost(&self) -> CostBreakdown {
            CostBreakdown::from_components(
                CostFormula {
                    display: "seconds × rate",
                    rates: vec![RateRow {
                        dimension: "—",
                        value: "any".into(),
                        unit: CostUnit::PerVideoSecond,
                        price_micros: 100_000,
                    }],
                },
                vec![CostComponent::new(
                    "video",
                    CostUnit::PerVideoSecond,
                    100_000,
                    CostQuantity::Exact(self.seconds),
                )],
                false,
            )
        }
    }

    #[test]
    fn a_request_prices_itself_as_a_breakdown() {
        let breakdown = FakeVideoRequest { seconds: 4.0 }.estimate_cost();
        assert_eq!(breakdown.amount, CostAmount::Exact(MicroUsd(400_000)));
        assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
        assert_eq!(breakdown.components.len(), 1);
        assert_eq!(breakdown.formula.rates.len(), 1);
    }
}
