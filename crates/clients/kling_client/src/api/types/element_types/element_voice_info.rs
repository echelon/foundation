use serde::Deserialize;

/// The custom voice bound to an element (present when one was attached
/// explicitly via `element_voice_id`, or auto-created from a reference
/// video's human voice).
#[derive(Clone, Debug, Deserialize)]
pub struct ElementVoiceInfo {
  /// Globally unique custom voice id.
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
