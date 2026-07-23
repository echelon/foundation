use crate::cost::rate_row::RateRow;

/// How a model's price is computed: a human-readable formula plus the complete
/// rate card behind it.
#[derive(Clone, Debug)]
pub struct CostFormula {
    /// e.g. `"seconds × rate(resolution)"`, `"images × tier(image_size)"`,
    /// `"input_tokens×rate + output_tokens×rate (thinking bills as output)"`.
    pub display: &'static str,
    pub rates: Vec<RateRow>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::cost_unit::CostUnit;

    #[test]
    fn holds_a_rate_card() {
        let formula = CostFormula {
            display: "images × tier(image_size)",
            rates: vec![
                RateRow { dimension: "image_size", value: "1K".into(), unit: CostUnit::PerImage, price_micros: 67_000 },
                RateRow { dimension: "image_size", value: "4K".into(), unit: CostUnit::PerImage, price_micros: 151_000 },
            ],
        };
        assert_eq!(formula.rates.len(), 2);
    }
}
