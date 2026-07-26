use serde::Deserialize;

/// A custom (or preset) voice as returned by the voice management endpoints'
/// `task_result.voices` arrays.
#[derive(Clone, Debug, Deserialize)]
pub struct VoiceRecord {
  /// Globally unique custom voice id — what video requests reference.
  #[serde(default)]
  pub voice_id: Option<String>,
  #[serde(default)]
  pub voice_name: Option<String>,
  /// Trial audio download URL.
  #[serde(default)]
  pub trial_url: Option<String>,
  /// `"kling"` for the official library, otherwise a creator id.
  #[serde(default)]
  pub owned_by: Option<String>,
}
