use serde::Deserialize;

/// One entry of a task's `billing` array — what the task actually cost.
///
/// `charge_type` is `"cash"` (balance deduction; `amount` is the discounted
/// price, `list_price`/`currency` present) or `"unit"` (resource package
/// deduction; `amount` is units, `package_type` is `video`/`image`/`audio`).
/// All money/unit quantities arrive as decimal strings. This is the
/// authoritative post-hoc cost — the pre-call `CostCalculator` estimates
/// should reconcile against it.
#[derive(Clone, Debug, Deserialize)]
pub struct BillingEntry {
  #[serde(default)]
  pub charge_type: Option<String>,
  #[serde(default)]
  pub amount: Option<String>,
  #[serde(default)]
  pub package_type: Option<String>,
  #[serde(default)]
  pub list_price: Option<String>,
  /// Only on callback payloads, cash deductions: `CNY` or `USD`.
  #[serde(default)]
  pub currency: Option<String>,
}
