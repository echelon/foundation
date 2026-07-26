use serde::Deserialize;

/// Balance (cash) deduction detail on legacy task responses. Decimal strings.
#[derive(Clone, Debug, Deserialize)]
pub struct LegacyBalanceDeduction {
  /// The discounted price actually deducted from the balance.
  #[serde(default)]
  pub quota: Option<String>,
  /// The list price before discount.
  #[serde(default)]
  pub list_price: Option<String>,
}
