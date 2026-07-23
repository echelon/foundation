//! Money newtypes, one unit per file (`units/`). USD amounts are micro-USD
//! ([`MicroUsd`]); catalog per-unit rates are nano-USD ([`NanoUsd`]). One
//! rounding function so every charge rounds identically
//! (conventions_database.md §2).

pub mod rounding;
pub mod units;

pub use units::micro_usd::MicroUsd;
pub use units::nano_usd::NanoUsd;

/// Transitional alias for the pre-rename type name — new code writes
/// [`MicroUsd`]; this dies once nothing spells `Micros` anymore.
pub use units::micro_usd::MicroUsd as Micros;
