use serde::Deserialize;

/// A generated audio clip in a legacy `task_result.audios` array (audio
/// generation endpoints). Note the legacy field naming is `url_mp3` — the
/// new-style task outputs flip it to `mp3_url`.
#[derive(Clone, Debug, Deserialize)]
pub struct LegacyAudioResult {
  #[serde(default)]
  pub id: Option<String>,
  #[serde(default)]
  pub url_mp3: Option<String>,
  #[serde(default)]
  pub url_wav: Option<String>,
  /// Durations in seconds — decimal strings.
  #[serde(default)]
  pub duration_mp3: Option<String>,
  #[serde(default)]
  pub duration_wav: Option<String>,
}
