use money::MicroUsd;

use crate::cost::cost_amount::CostAmount;
use crate::cost::cost_component::CostComponent;
use crate::cost::cost_formula::CostFormula;

/// A request's full price picture: the (exact-or-estimated) total, the lines
/// behind it, and the model's formula/rate card.
#[derive(Clone, Debug)]
pub struct CostBreakdown {
    pub amount: CostAmount,
    /// The lines summing (±rounding) to `amount`.
    pub components: Vec<CostComponent>,
    pub formula: CostFormula,
}

impl CostBreakdown {
    /// Assemble from components: the total is their sum; it is `Exact` only if
    /// EVERY component quantity is exact and `force_estimated` is false (set
    /// that when the provider can bill beyond the components, e.g. thinking
    /// tokens on a per-image rate).
    pub fn from_components(formula: CostFormula, components: Vec<CostComponent>, force_estimated: bool) -> Self {
        let total: i64 = components.iter().map(|c| c.subtotal_micros).sum();
        let estimated = force_estimated || components.iter().any(|c| c.quantity.is_estimated());
        let amount = if estimated { CostAmount::Estimated(MicroUsd(total)) } else { CostAmount::Exact(MicroUsd(total)) };
        Self { amount, components, formula }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::cost_certainty::CostCertainty;
    use crate::cost::cost_quantity::CostQuantity;
    use crate::cost::cost_unit::CostUnit;

    fn flat(label: &str, price: i64, quantity: CostQuantity) -> CostComponent {
        CostComponent::new(label, CostUnit::PerRequest, price, quantity)
    }

    fn formula() -> CostFormula {
        CostFormula { display: "test", rates: vec![] }
    }

    #[test]
    fn totals_sum_and_exactness_derives_from_quantities() {
        let b = CostBreakdown::from_components(
            formula(),
            vec![flat("a", 10, CostQuantity::Exact(1.0)), flat("b", 5, CostQuantity::Exact(2.0))],
            false,
        );
        assert_eq!(b.amount, CostAmount::Exact(MicroUsd(20)));
    }

    #[test]
    fn any_estimated_quantity_makes_the_total_estimated() {
        let b = CostBreakdown::from_components(
            formula(),
            vec![flat("a", 10, CostQuantity::Exact(1.0)), flat("b", 5, CostQuantity::Estimated(2.0))],
            false,
        );
        assert_eq!(b.amount.certainty(), CostCertainty::Estimated);
    }

    #[test]
    fn force_estimated_overrides_exact_quantities() {
        let b = CostBreakdown::from_components(formula(), vec![flat("a", 10, CostQuantity::Exact(1.0))], true);
        assert_eq!(b.amount.certainty(), CostCertainty::Estimated);
    }
}
