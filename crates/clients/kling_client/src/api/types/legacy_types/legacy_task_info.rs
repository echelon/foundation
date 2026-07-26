use serde::Deserialize;

use crate::api::types::legacy_types::legacy_video_result::LegacyVideoResult;

/// The `task_info` object on legacy task responses — creation parameters
/// echoed back.
#[derive(Clone, Debug, Deserialize)]
pub struct LegacyTaskInfo {
  /// The caller-chosen correlation id (if one was set at create time).
  #[serde(default)]
  pub external_task_id: Option<String>,
  /// Only on tasks derived from an existing video (e.g. video-to-audio): the
  /// source video.
  #[serde(default)]
  pub parent_video: Option<LegacyVideoResult>,
}
