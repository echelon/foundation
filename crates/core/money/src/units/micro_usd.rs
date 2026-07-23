use serde::{Deserialize, Serialize};

use crate::rounding::round_half_up;

/// Micro-USD (1e-6 USD). The unit for balances, charges, and ledger amounts
/// (every `_micros` column/field).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MicroUsd(pub i64);

impl MicroUsd {
    pub const ZERO: MicroUsd = MicroUsd(0);

    pub fn from_usd(usd: f64) -> MicroUsd {
        MicroUsd(round_half_up(usd * 1_000_000.0))
    }

    pub fn as_usd(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usd_roundtrip() {
        assert_eq!(MicroUsd::from_usd(0.02), MicroUsd(20_000));
        assert_eq!(MicroUsd::from_usd(1.5).as_usd(), 1.5);
        assert_eq!(MicroUsd::ZERO, MicroUsd(0));
    }

    #[test]
    fn from_usd_rounds_half_up() {
        // 0.0000005 USD = 0.5 micros → rounds up to 1.
        assert_eq!(MicroUsd::from_usd(0.000_000_5), MicroUsd(1));
    }

    #[test]
    fn orders_and_serializes_transparently() {
        assert!(MicroUsd(1) < MicroUsd(2));
        assert_eq!(serde_json::to_string(&MicroUsd(42)).unwrap(), "42");
    }
}
