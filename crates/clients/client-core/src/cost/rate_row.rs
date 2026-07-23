use crate::cost::cost_unit::CostUnit;

/// One row of a model's rate card: the price that applies when `dimension`
/// takes `value` (e.g. dimension "resolution", value "720p"). A frontend can
/// render the full card as a table, or drive a calculator from it.
#[derive(Clone, Debug)]
pub struct RateRow {
    /// The request knob that selects this rate, e.g. `"image_size"`,
    /// `"resolution"`, `"tokens"`; `"—"` when the model has one flat rate.
    pub dimension: &'static str,
    /// The knob's value this row prices, e.g. `"2K"`, `"1080p"`, `"input"`.
    pub value: String,
    pub unit: CostUnit,
    pub price_micros: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_a_priced_dimension_value() {
        let row =
            RateRow { dimension: "resolution", value: "720p".into(), unit: CostUnit::PerVideoSecond, price_micros: 100_000 };
        assert_eq!(row.dimension, "resolution");
        assert_eq!(row.unit.as_str(), "per_video_second");
    }
}
