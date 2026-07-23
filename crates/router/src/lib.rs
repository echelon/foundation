//! foundation_router — the hard-coded routing layer between our Any-shaped APIs and the
//! provider clients (modeled on artcraft_router, radically simplified: no dynamic provider
//! selection — each catalog model routes to exactly one hard-coded implementation).
//!
//! Shape (congruent across models so the Any plane stays uniform):
//!   - [`image::plan::ImageGenerationPlan`] — the provider-agnostic superset request;
//!   - `build_image_route(plan)` — per-model build fns prune/coerce params (recorded as
//!     [`image::adjustments::ParamAdjustment`], the api_any.md `param_adjustments`) and
//!     fan out `n` into N provider-native requests (OpenAI's image tool has no `n`);
//!   - [`pricing::PriceQuote`] — USD-micros pricing for ANY request, delegated to the
//!     provider client's own `CostCalculator` (never re-derived), with certainty labeling
//!     (exact / estimated / post-billed);
//!   - [`input_strategy::InputStrategy`] — whether a provider takes our public URLs
//!     (OpenAI: yes) or needs assets uploaded to it first (Kinovi-style, future);
//!   - `start_*` returns provider handles; `poll_*` fetches slice results. The router never
//!     holds a request open and never polls on its own.

pub mod error;
pub mod image;
pub mod video;
pub mod input_strategy;
pub mod pricing;
