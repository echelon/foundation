//! Shared plumbing for provider clients (codebase_layout_backend.md §5).
//!
//! Deliberately lean — the cross-cutting pieces live here ONCE so the per-service client
//! crates don't each re-declare them:
//!   - [`http_client`]    — the reqwest factory with our provider-call timeouts.
//!   - [`CostCalculator`] — one cost interface, in `money::Micros`, that every request type
//!                          implements (next to its endpoint, in `cost.rs`).
//!
//! Everything else — errors, credentials, endpoint bindings, typed enums, and each client's
//! own success/response types — is provider-specific and lives in the `{service}_client`
//! crate. (Normalized cross-provider output — "an image is bytes, a video is a URL, a job is
//! pending/done" — belongs to the future dispatch/`router` layer, not to this plumbing crate.)

use std::time::Duration;

pub use money::Micros;

/// Build a reqwest client with our provider-call defaults (connect + total timeouts).
pub fn http_client(user_agent: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .build()
        .expect("build reqwest client")
}

/// How binding `cost_micros()` is to the final bill.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CostCertainty {
    /// The request params fully determine the charge.
    Exact,
    /// Good-faith estimate — the provider's reported usage is authoritative
    /// (e.g. thinking tokens billed on top of a per-image rate).
    Estimated,
}

/// Estimate the cost of a request WITHOUT making it. Implemented on each `*Request` type
/// (the request body is what drives the bill), next to the endpoint in `cost.rs`.
pub trait CostCalculator {
    /// Estimated charge in micro-USD (`money::Micros`).
    fn cost_micros(&self) -> Micros;

    /// Whether `cost_micros()` is exact or an estimate. Defaults to exact —
    /// override where the provider can bill beyond what the params determine.
    fn cost_certainty(&self) -> CostCertainty {
        CostCertainty::Exact
    }
}
