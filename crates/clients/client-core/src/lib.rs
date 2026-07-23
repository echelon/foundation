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
///
/// DEPRECATED-IN-SPIRIT: new code should implement [`BetterCostEstimator`] (which carries
/// certainty, a renderable formula, and a component breakdown); this trait remains until
/// every client crate has migrated.
pub trait CostCalculator {
    /// Estimated charge in micro-USD (`money::Micros`).
    fn cost_micros(&self) -> Micros;

    /// Whether `cost_micros()` is exact or an estimate. Defaults to exact —
    /// override where the provider can bill beyond what the params determine.
    fn cost_certainty(&self) -> CostCertainty {
        CostCertainty::Exact
    }
}

// ── BetterCostEstimator ─────────────────────────────────────────────────────
//
// The richer replacement for `CostCalculator`: instead of one opaque number, a request
// prices itself as a BREAKDOWN — an exact-or-estimated total, the per-line components
// behind it, and the model's full rate card + formula (structured enough for a pricing
// table or an interactive frontend calculator). One trait covers the pricing shapes of
// LLMs (per-token in/out), image models (per-image, possibly tiered), video models
// (per-second by resolution, or token-priced), and music (per-song).

/// Billing unit for one priced component / rate-card row.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CostUnit {
    /// Flat per request (e.g. Lyria's per-song price).
    PerRequest,
    /// Per output image.
    PerImage,
    /// Per second of output video.
    PerVideoSecond,
    /// Per 1M input tokens.
    PerMillionInputTokens,
    /// Per 1M output tokens (providers usually bill thinking tokens here).
    PerMillionOutputTokens,
}

impl CostUnit {
    /// Stable machine-readable name (frontends key labels/formatting off this).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PerRequest => "per_request",
            Self::PerImage => "per_image",
            Self::PerVideoSecond => "per_video_second",
            Self::PerMillionInputTokens => "per_million_tokens_in",
            Self::PerMillionOutputTokens => "per_million_tokens_out",
        }
    }

    /// How many units one "price" covers (1M for token units, 1 otherwise) —
    /// the divisor when turning `quantity × price` into a subtotal.
    fn units_per_price(&self) -> f64 {
        match self {
            Self::PerMillionInputTokens | Self::PerMillionOutputTokens => 1_000_000.0,
            _ => 1.0,
        }
    }
}

/// A quantity entering the bill: fixed by the request params, or estimated
/// pre-call (e.g. token counts, prompt-driven output lengths).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CostQuantity {
    Exact(f64),
    Estimated(f64),
}

impl CostQuantity {
    pub fn value(&self) -> f64 {
        match self {
            Self::Exact(v) | Self::Estimated(v) => *v,
        }
    }

    pub fn is_estimated(&self) -> bool {
        matches!(self, Self::Estimated(_))
    }
}

/// One line of the bill: `quantity` of `unit` at `unit_price_micros`.
#[derive(Clone, Debug)]
pub struct CostComponent {
    /// Human label, e.g. `"Output image (2K tier)"`, `"Input tokens"`.
    pub label: String,
    pub unit: CostUnit,
    /// Price per unit-batch in micro-USD (for token units: per 1M tokens).
    pub unit_price_micros: i64,
    pub quantity: CostQuantity,
    /// `quantity × unit_price` (÷ 1M for token units), rounded to a micro.
    pub subtotal_micros: i64,
    /// Caveats a UI should surface, e.g. "thinking tokens bill as output".
    pub note: Option<String>,
}

impl CostComponent {
    /// Build a component with the subtotal computed from unit semantics.
    pub fn new(label: impl Into<String>, unit: CostUnit, unit_price_micros: i64, quantity: CostQuantity) -> Self {
        let subtotal = (quantity.value() * unit_price_micros as f64 / unit.units_per_price()).round() as i64;
        Self { label: label.into(), unit, unit_price_micros, quantity, subtotal_micros: subtotal, note: None }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// One row of a model's rate card: the price that applies when `dimension`
/// takes `value` (e.g. dimension "resolution", value "720p"). A frontend can
/// render the full card as a table, or drive a calculator from it.
#[derive(Clone, Debug)]
pub struct RateRow {
    /// The request knob that selects this rate, e.g. `"image_size"`,
    /// `"resolution"`, `"tokens"`; `"—"` when the model has one flat rate.
    pub dimension: &'static str,
    /// The knob's value this row prices, e.g. `"2K"`, `"1080p"`, `"input"`.
    pub value: String,
    pub unit: CostUnit,
    pub price_micros: i64,
}

/// How a model's price is computed: a human-readable formula plus the complete
/// rate card behind it.
#[derive(Clone, Debug)]
pub struct CostFormula {
    /// e.g. `"seconds × rate(resolution)"`, `"images × tier(image_size)"`,
    /// `"input_tokens×rate + output_tokens×rate (thinking bills as output)"`.
    pub display: &'static str,
    pub rates: Vec<RateRow>,
}

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

/// A request's full price picture.
#[derive(Clone, Debug)]
pub struct CostBreakdown {
    pub amount: CostAmount,
    /// The lines summing (±rounding) to `amount`.
    pub components: Vec<CostComponent>,
    pub formula: CostFormula,
}

impl CostBreakdown {
    /// Assemble from components: the total is their sum; it is `Exact` only if
    /// EVERY component quantity is exact and `any_estimated_rates` is false.
    pub fn from_components(formula: CostFormula, components: Vec<CostComponent>, force_estimated: bool) -> Self {
        let total: i64 = components.iter().map(|c| c.subtotal_micros).sum();
        let estimated = force_estimated || components.iter().any(|c| c.quantity.is_estimated());
        let amount = if estimated { CostAmount::Estimated(Micros(total)) } else { CostAmount::Exact(Micros(total)) };
        Self { amount, components, formula }
    }
}

/// The richer pricing interface (successor to [`CostCalculator`]): certainty,
/// a concrete-or-estimated total, per-component lines, and a renderable
/// formula/rate card. Implemented on each `*Request` next to its endpoint.
pub trait BetterCostEstimator {
    fn estimate_cost(&self) -> CostBreakdown;
}
