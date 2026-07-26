use serde::Deserialize;

use crate::api::types::legacy_types::legacy_balance_deduction::LegacyBalanceDeduction;
use crate::api::types::legacy_types::legacy_task_info::LegacyTaskInfo;
use crate::api::types::legacy_types::legacy_task_status::LegacyTaskStatus;

/// A full task record from a legacy-style `/v1/*` query endpoint. `TResult`
/// is the family-specific `task_result` payload (e.g. `{images: […]}` for
/// image generation, `{elements: […]}` for element management).
///
/// A failed task is an `Ok(LegacyTask)` with `task_status == Failed` and the
/// reason in `task_status_msg` — only transport/HTTP problems are `Err` at
/// the binding layer.
#[derive(Clone, Debug, Deserialize)]
pub struct LegacyTask<TResult> {
  pub task_id: String,
  pub task_status: LegacyTaskStatus,
  #[serde(default)]
  pub task_status_msg: Option<String>,
  #[serde(default)]
  pub task_info: Option<LegacyTaskInfo>,
  /// Present once the task has produced results (normally on `succeed`).
  // No `#[serde(default)]` here: on a generic field it would force a
  // `TResult: Default` bound, and serde already treats a missing `Option` as
  // None.
  pub task_result: Option<TResult>,
  /// Units deducted from a resource package — a decimal string.
  #[serde(default)]
  pub final_unit_deduction: Option<String>,
  #[serde(default)]
  pub final_balance_deduction: Option<LegacyBalanceDeduction>,
  /// Unix timestamps in milliseconds.
  #[serde(default)]
  pub created_at: Option<i64>,
  #[serde(default)]
  pub updated_at: Option<i64>,
}
