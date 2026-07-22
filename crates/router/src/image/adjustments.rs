//! Param adjustments (api_any.md): when a plan asks for something the routed model can't
//! honor, we prune/coerce and RECORD it — reported in the response and stored on the job.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ParamAdjustment {
  pub param: String,
  pub action: AdjustmentAction,
  pub detail: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentAction {
  /// The param was dropped (model doesn't support it).
  Pruned,
  /// The value was changed to the nearest supported one.
  Coerced,
}

impl ParamAdjustment {
  pub fn pruned(param: &str, detail: impl Into<String>) -> Self {
    Self { param: param.into(), action: AdjustmentAction::Pruned, detail: detail.into() }
  }
  pub fn coerced(param: &str, detail: impl Into<String>) -> Self {
    Self { param: param.into(), action: AdjustmentAction::Coerced, detail: detail.into() }
  }
}
