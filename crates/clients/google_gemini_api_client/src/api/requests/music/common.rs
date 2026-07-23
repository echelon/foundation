//! Shared response mapping for the Lyria music bindings.

use crate::api::requests::interactions::common::InteractionSuccess;
use crate::error::google_gemini_error::GoogleGeminiError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;

/// One generated track, decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedAudio {
  /// MIME type, e.g. `audio/mpeg` (MP3 default) or `audio/wav`.
  pub mime: String,
  /// Decoded audio bytes.
  pub bytes: Vec<u8>,
}

/// The outcome of one music generation.
#[derive(Clone, Debug)]
pub struct GenerateMusicSuccess {
  /// One entry per generated track (normally one).
  pub audio: Vec<GeneratedAudio>,
  /// The lyrics, when the model wrote and returned them as text.
  pub lyrics: Option<String>,
  /// Pass back as `previous_interaction_id` for follow-up edits.
  pub interaction_id: Option<String>,
}

/// Map a COMPLETED interaction's items into music output (used after a
/// successful poll).
pub fn music_success_from_interaction(
  interaction: InteractionSuccess,
) -> Result<GenerateMusicSuccess, GoogleGeminiError> {
  let lyrics = interaction.joined_text();
  let audio: Vec<GeneratedAudio> = interaction
    .items
    .iter()
    .filter(|i| i.kind == "audio")
    .filter_map(|i| {
      i.bytes.as_ref().map(|b| GeneratedAudio {
        mime: i.mime.clone().unwrap_or_else(|| "audio/mpeg".to_string()),
        bytes: b.clone(),
      })
    })
    .collect();
  if audio.is_empty() {
    return Err(
      GoogleGeminiGenericApiError::EmptyResponse {
        detail: "interaction succeeded but carried no inline audio items".to_string(),
      }
      .into(),
    );
  }
  Ok(GenerateMusicSuccess { audio, lyrics, interaction_id: interaction.interaction_id })
}
