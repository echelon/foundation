//! Money newtypes. USD amounts are micro-USD (`Micros`); catalog unit rates are
//! nano-USD (`NanoUsd`). One rounding function so every charge rounds identically
//! (conventions_database.md §2).

use serde::{Deserialize, Serialize};

/// Micro-USD (1e-6 USD). The unit for balances, charges, ledger amounts (`_micros`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Micros(pub i64);

/// Nano-USD (1e-9 USD). The unit for catalog per-unit rates (`_nanos`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NanoUsd(pub i64);

impl Micros {
    pub const ZERO: Micros = Micros(0);
    pub fn from_usd(usd: f64) -> Micros {
        Micros(round_half_up(usd * 1_000_000.0))
    }
    pub fn as_usd(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }
}

impl NanoUsd {
    /// Charge = `rate_nanos * quantity`, rounded to whole micro-USD (the ONE rounding fn).
    pub fn charge(self, quantity: f64) -> Micros {
        // nanos * qty is in nano-USD; divide by 1000 to land in micro-USD.
        Micros(round_half_up(self.0 as f64 * quantity / 1000.0))
    }
}

/// Round-half-up to the nearest integer. The single rounding rule for all money.
fn round_half_up(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usd_roundtrip() {
        assert_eq!(Micros::from_usd(0.02), Micros(20_000));
        assert_eq!(Micros::from_usd(1.5).as_usd(), 1.5);
    }

    #[test]
    fn metered_charge_rounds_once() {
        // $0.12 / video-second = 120_000_000 nanos; 5s → $0.60 → 600_000 micros
        let rate = NanoUsd(120_000_000);
        assert_eq!(rate.charge(5.0), Micros(600_000));
    }
}
