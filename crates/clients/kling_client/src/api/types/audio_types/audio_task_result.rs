use serde::Deserialize;

use crate::api::types::legacy_types::legacy_audio_result::LegacyAudioResult;
use crate::api::types::legacy_types::legacy_video_result::LegacyVideoResult;

/// The `task_result` payload of audio generation tasks. Text-to-audio fills
/// `audios` only; video-to-audio fills both `audios` (the generated sound)
/// and `videos` (the source video remuxed with it).
#[derive(Clone, Debug, Deserialize)]
pub struct AudioTaskResult {
  #[serde(default)]
  pub audios: Vec<LegacyAudioResult>,
  #[serde(default)]
  pub videos: Vec<LegacyVideoResult>,
}
