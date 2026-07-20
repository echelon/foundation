//! Shared plumbing for provider clients (codebase_layout_backend.md §5).
//!
//! Deliberately lean — the *improvement* over ArtCraft's per-client duplication is that
//! the cross-cutting pieces live here ONCE:
//!   - [`http_client`]      — the reqwest factory with our provider-call timeouts.
//!   - [`CostCalculator`]   — one cost interface, in `money::Micros`, that every request
//!                            type implements (ArtCraft re-declared a bespoke trait per client).
//!   - [`Asset`] / [`JobState`] — the normalized generation output the dispatcher stores,
//!                            so sync (image) and async (video) providers converge.
//!
//! Each client still owns its OWN rich error module, credentials, endpoint bindings, and
//! typed enums — those are provider-specific and belong next to the endpoints.

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

/// Estimate the cost of a request WITHOUT making it. Implemented on each `*Request` type
/// (the request body is what drives the bill), next to the endpoint in `cost.rs`.
pub trait CostCalculator {
    /// Estimated charge in micro-USD (`money::Micros`).
    fn cost_micros(&self) -> Micros;
}

/// A normalized generation output the dispatcher stores, regardless of provider shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asset {
    /// Bytes returned inline (OpenAI/Gemini image base64 decoded).
    Bytes { mime: String, data: Vec<u8> },
    /// A URL to fetch (Kinovi/Seedance video).
    Url { mime: Option<String>, url: String },
}

/// Where an async provider job is in its lifecycle (Kinovi-style poll providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl JobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, JobState::Succeeded | JobState::Failed)
    }
}
