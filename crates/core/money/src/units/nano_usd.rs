use serde::{Deserialize, Serialize};

use crate::rounding::round_half_up;
use crate::units::micro_usd::MicroUsd;

/// Nano-USD (1e-9 USD). The unit for catalog per-unit rates (every `_nanos`
/// column/field) — fine enough to price fractions of a token without losing
/// integer exactness.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NanoUsd(pub i64);

impl NanoUsd {
    /// Charge = `rate_nanos × quantity`, rounded ONCE to whole micro-USD (the
    /// single rounding step every metered charge goes through).
    pub fn charge(self, quantity: f64) -> MicroUsd {
        // nanos × qty is in nano-USD; divide by 1000 to land in micro-USD.
        MicroUsd(round_half_up(self.0 as f64 * quantity / 1000.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metered_charge_rounds_once() {
        // $0.12 / video-second = 120_000_000 nanos; 5s → $0.60 → 600_000 micros.
        assert_eq!(NanoUsd(120_000_000).charge(5.0), MicroUsd(600_000));
    }

    #[test]
    fn sub_micro_rates_round_at_the_end() {
        // 1 nano × 1499 units = 1.499 micros → 1 micro (half-up applied once).
        assert_eq!(NanoUsd(1).charge(1499.0), MicroUsd(1));
        assert_eq!(NanoUsd(1).charge(1500.0), MicroUsd(2));
    }
}
