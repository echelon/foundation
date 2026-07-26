use serde::Deserialize;

use crate::api::types::voice_types::voice_record::VoiceRecord;

/// The `task_result` payload of voice management tasks.
#[derive(Clone, Debug, Deserialize)]
pub struct VoiceTaskResult {
  #[serde(default)]
  pub voices: Vec<VoiceRecord>,
}
