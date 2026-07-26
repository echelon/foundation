use serde::Deserialize;

/// A generated video in a legacy `task_result.videos` array (also the
/// `parent_video` shape on video-to-audio tasks). URLs are purged upstream
/// after 30 days.
#[derive(Clone, Debug, Deserialize)]
pub struct LegacyVideoResult {
  #[serde(default)]
  pub id: Option<String>,
  pub url: String,
  /// Duration in seconds — a decimal string.
  #[serde(default)]
  pub duration: Option<String>,
}
