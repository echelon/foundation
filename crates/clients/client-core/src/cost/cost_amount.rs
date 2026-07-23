use money::MicroUsd;

use crate::cost::cost_certainty::CostCertainty;

/// The priced total: exact when the params fully determine the charge,
/// estimated when the provider's post-call usage is authoritative.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CostAmount {
    Exact(MicroUsd),
    Estimated(MicroUsd),
}

impl CostAmount {
    pub fn micros(&self) -> MicroUsd {
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
        assert_eq!(CostAmount::Exact(MicroUsd(5)).micros(), MicroUsd(5));
        assert_eq!(CostAmount::Exact(MicroUsd(5)).certainty(), CostCertainty::Exact);
        assert_eq!(CostAmount::Estimated(MicroUsd(9)).certainty(), CostCertainty::Estimated);
    }
}
