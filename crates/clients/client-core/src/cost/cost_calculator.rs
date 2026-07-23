use money::Micros;

use crate::cost::cost_certainty::CostCertainty;

/// LEGACY: estimate the cost of a request WITHOUT making it — one opaque
/// number plus a certainty flag. New code implements
/// [`crate::cost::better_cost_estimator::BetterCostEstimator`] instead (which
/// adds components and a renderable rate card); this trait remains until every
/// client crate has migrated.
pub trait CostCalculator {
    /// Estimated charge in micro-USD (`money::Micros`).
    fn cost_micros(&self) -> Micros;

    /// Whether `cost_micros()` is exact or an estimate. Defaults to exact —
    /// override where the provider can bill beyond what the params determine.
    fn cost_certainty(&self) -> CostCertainty {
        CostCertainty::Exact
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat-priced request exercising the interface + the default certainty.
    struct FlatRequest;

    impl CostCalculator for FlatRequest {
        fn cost_micros(&self) -> Micros {
            Micros(1_250)
        }
    }

    struct DriftingRequest;

    impl CostCalculator for DriftingRequest {
        fn cost_micros(&self) -> Micros {
            Micros(99)
        }

        fn cost_certainty(&self) -> CostCertainty {
            CostCertainty::Estimated
        }
    }

    #[test]
    fn interface_and_default_certainty() {
        assert_eq!(FlatRequest.cost_micros(), Micros(1_250));
        assert_eq!(FlatRequest.cost_certainty(), CostCertainty::Exact);
        assert_eq!(DriftingRequest.cost_certainty(), CostCertainty::Estimated);
    }
}
