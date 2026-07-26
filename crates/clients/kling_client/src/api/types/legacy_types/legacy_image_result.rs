use serde::Deserialize;

/// A generated image in a legacy `task_result.images` array. URLs are purged
/// upstream after 30 days.
#[derive(Clone, Debug, Deserialize)]
pub struct LegacyImageResult {
  /// Image number within the batch (0-based, up to `n - 1`).
  #[serde(default)]
  pub index: Option<i64>,
  pub url: String,
  #[serde(default)]
  pub watermark_url: Option<String>,
}
