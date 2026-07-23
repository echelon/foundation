/// A quantity entering the bill: fixed by the request params, or estimated
/// pre-call (e.g. token counts, prompt-driven output lengths).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CostQuantity {
    Exact(f64),
    Estimated(f64),
}

impl CostQuantity {
    pub fn value(&self) -> f64 {
        match self {
            Self::Exact(v) | Self::Estimated(v) => *v,
        }
    }

    pub fn is_estimated(&self) -> bool {
        matches!(self, Self::Estimated(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_and_estimatedness() {
        assert_eq!(CostQuantity::Exact(8.0).value(), 8.0);
        assert_eq!(CostQuantity::Estimated(512.0).value(), 512.0);
        assert!(!CostQuantity::Exact(1.0).is_estimated());
        assert!(CostQuantity::Estimated(1.0).is_estimated());
    }
}
