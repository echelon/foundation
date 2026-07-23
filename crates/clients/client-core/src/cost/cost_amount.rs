use money::Micros;

use crate::cost::cost_certainty::CostCertainty;

/// The priced total: exact when the params fully determine the charge,
/// estimated when the provider's post-call usage is authoritative.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CostAmount {
    Exact(Micros),
    Estimated(Micros),
}

impl CostAmount {
    pub fn micros(&self) -> Micros {
        match self {
            Self::Exact(m) | Self::Estimated(m) => *m,
        }
    }

    pub fn certainty(&self) -> CostCertainty {
        match self {
            Self::Exact(_) => CostCertainty::Exact,
            Self::Estimated(_) => CostCertainty::Estimated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_carries_certainty() {
        assert_eq!(CostAmount::Exact(Micros(5)).micros(), Micros(5));
        assert_eq!(CostAmount::Exact(Micros(5)).certainty(), CostCertainty::Exact);
        assert_eq!(CostAmount::Estimated(Micros(9)).certainty(), CostCertainty::Estimated);
    }
}
