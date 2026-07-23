//! Shared plumbing for provider clients (codebase_layout_backend.md §5).
//!
//! Deliberately lean — the cross-cutting pieces live here ONCE so the per-service client
//! crates don't each re-declare them:
//!   - [`http_client`]          — the reqwest factory with our provider-call timeouts.
//!   - [`cost`]                 — the pricing vocabulary: [`BetterCostEstimator`] (the
//!                                current interface: certainty + components + rate card)
//!                                and the legacy [`CostCalculator`] it is replacing.
//!
//! Everything else — errors, credentials, endpoint bindings, typed enums, and each client's
//! own success/response types — is provider-specific and lives in the `{service}_client`
//! crate.

use std::time::Duration;

pub mod cost;

pub use cost::better_cost_estimator::BetterCostEstimator;
pub use cost::cost_amount::CostAmount;
pub use cost::cost_breakdown::CostBreakdown;
pub use cost::cost_calculator::CostCalculator;
pub use cost::cost_certainty::CostCertainty;
pub use cost::cost_component::CostComponent;
pub use cost::cost_formula::CostFormula;
pub use cost::cost_quantity::CostQuantity;
pub use cost::cost_unit::CostUnit;
pub use cost::rate_row::RateRow;
pub use money::MicroUsd;

/// Build a reqwest client with our provider-call defaults (connect + total timeouts).
pub fn http_client(user_agent: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .build()
        .expect("build reqwest client")
}
