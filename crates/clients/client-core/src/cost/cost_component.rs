use crate::cost::cost_quantity::CostQuantity;
use crate::cost::cost_unit::CostUnit;

/// One line of the bill: `quantity` of `unit` at `unit_price_micros`.
#[derive(Clone, Debug)]
pub struct CostComponent {
    /// Human label, e.g. `"Output image (2K tier)"`, `"Input tokens"`.
    pub label: String,
    pub unit: CostUnit,
    /// Price per unit-batch in micro-USD (for token units: per 1M tokens).
    pub unit_price_micros: i64,
    pub quantity: CostQuantity,
    /// `quantity × unit_price` (÷ 1M for token units), rounded to a micro.
    pub subtotal_micros: i64,
    /// Caveats a UI should surface, e.g. "thinking tokens bill as output".
    pub note: Option<String>,
}

impl CostComponent {
    /// Build a component with the subtotal computed from unit semantics.
    pub fn new(label: impl Into<String>, unit: CostUnit, unit_price_micros: i64, quantity: CostQuantity) -> Self {
        let subtotal = (quantity.value() * unit_price_micros as f64 / unit.units_per_price()).round() as i64;
        Self { label: label.into(), unit, unit_price_micros, quantity, subtotal_micros: subtotal, note: None }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_unit_subtotal() {
        // 8 seconds × $0.40/s = $3.20
        let c = CostComponent::new("video", CostUnit::PerVideoSecond, 400_000, CostQuantity::Exact(8.0));
        assert_eq!(c.subtotal_micros, 3_200_000);
        assert!(c.note.is_none());
    }

    #[test]
    fn per_million_token_subtotal_divides() {
        // 1000 tokens at $7.50/1M = $0.0075
        let c = CostComponent::new("out", CostUnit::PerMillionOutputTokens, 7_500_000, CostQuantity::Estimated(1000.0));
        assert_eq!(c.subtotal_micros, 7_500);
    }

    #[test]
    fn note_attaches() {
        let c = CostComponent::new("x", CostUnit::PerRequest, 1, CostQuantity::Exact(1.0)).with_note("caveat");
        assert_eq!(c.note.as_deref(), Some("caveat"));
    }
}
