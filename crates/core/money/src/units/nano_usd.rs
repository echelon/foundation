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

    /// PROOF vs exact integer arithmetic: for integer quantities, charge()
    /// must equal round_half_up(rate × qty / 1000) computed in i128 rationals
    /// (⌊(n×q + 500)/1000⌋ for non-negatives). Sweeps realistic rate shapes ×
    /// quantities, including every remainder class mod 1000 and exact halves.
    #[test]
    fn charge_matches_exact_integer_arithmetic() {
        let rates: Vec<i64> = (0..2000)
            .chain((0..200).map(|k| 999_000 + k))               // sub-micro rates, every remainder near 1M
            .chain([11_000_000, 20_000_000, 39_000_000, 120_000_000, 600_000_000, 17_500_000_000])
            .collect();
        let quantities: Vec<i64> = (0..50).chain([100, 999, 1_000, 1_001, 1_499, 1_500, 4_096, 1_000_000]).collect();
        for &rate in &rates {
            for &qty in &quantities {
                let exact = ((rate as i128) * (qty as i128) + 500) / 1000; // half-up in exact integers
                let got = NanoUsd(rate).charge(qty as f64);
                assert_eq!(got, MicroUsd(exact as i64), "rate={rate} qty={qty}");
            }
        }
    }

    #[test]
    fn sub_micro_rates_round_at_the_end() {
        // 1 nano × 1499 units = 1.499 micros → 1 micro (half-up applied once).
        assert_eq!(NanoUsd(1).charge(1499.0), MicroUsd(1));
        assert_eq!(NanoUsd(1).charge(1500.0), MicroUsd(2));
    }
}
