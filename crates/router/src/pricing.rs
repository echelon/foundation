//! Pricing quotes in USD micros for ANY routed request. Amounts are DELEGATED to the
//! provider client's `BetterCostEstimator` (router never re-derives prices — the artcraft_router
//! convention), then labeled with how trustworthy they are.

use serde::Serialize;

/// How certain the quoted amount is.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceCertainty {
  /// The bill will be exactly this.
  Exact,
  /// A good-faith estimate (e.g. token-metered image pricing at 1024²; larger sizes and
  /// edit inputs can bill more). The provider's post-call usage is authoritative.
  Estimated,
  /// Cannot be known up front — billed after the fact from provider usage.
  PostBilled,
}

#[derive(Clone, Debug, Serialize)]
pub struct PriceLineItem {
  pub label: String,
  pub amount_micros: i64,
}

/// The quote for one plan (all `n` fan-out slices included).
#[derive(Clone, Debug, Serialize)]
pub struct PriceQuote {
  pub certainty: PriceCertainty,
  /// Best-estimate total in USD micros.
  pub amount_micros: i64,
  /// Known upper bound, when the price is a range (None = no better bound than the estimate).
  pub max_micros: Option<i64>,
  pub line_items: Vec<PriceLineItem>,
}

impl PriceQuote {
  pub fn amount_display(&self) -> String {
    format!("${:.4}", self.amount_micros as f64 / 1_000_000.0)
  }
}
