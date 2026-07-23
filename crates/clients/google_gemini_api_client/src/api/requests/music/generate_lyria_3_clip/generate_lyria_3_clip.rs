//! `POST /v1beta/interactions` — **Lyria 3 Clip** (`lyria-3-clip-preview`), type `music`.
//! Short clips, loops, and previews — fixed ~30-second output. 44.1 kHz stereo; MP3 by default. Musical controls (BPM, key,
//! duration, structure) are steered through natural language in the prompt —
//! the API exposes no discrete knobs for them.
//!
//! ASYNC: starts a background interaction and returns the handle immediately —
//! poll with [`crate::api::requests::interactions::get_interaction`], then map
//! the completed items with [`music_success_from_interaction`]. No connection
//! is held while the model renders.
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/music-generation>

use serde::Serialize;

use crate::api::requests::interactions::common::{run_background_interaction, InputItem, InteractionRequestBody};
use crate::api::types::image_types::image_input::ImageInput;
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::google_gemini_error::GoogleGeminiError;

pub use crate::api::requests::interactions::common::InteractionHandle;
pub use crate::api::requests::music::common::{music_success_from_interaction, GeneratedAudio, GenerateMusicSuccess};

/// The exact model identifier this binding pins.
pub const LYRIA_3_CLIP_MODEL_ID: &str = "lyria-3-clip-preview";

/// Top-level argument to [`generate_lyria_3_clip`]. Borrows the API key separately
/// from the request body so callers can log/persist (and PRICE) the request
/// without touching the credential.
#[derive(Clone, Debug)]
pub struct GenerateLyria3ClipArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  pub request: Lyria3ClipRequest,
}

/// The material part of a Lyria 3 Clip request — drives both the wire body and the
/// [`client_core::BetterCostEstimator`] price (see `cost.rs`).
#[derive(Clone, Debug, Serialize)]
pub struct Lyria3ClipRequest {
  /// The music prompt — genre, mood, instrumentation, structure, BPM, key,
  /// lyrics direction. The more specific, the better.
  pub prompt: String,

  /// Optional inspiration images (≤ 10). Prefer [`ImageInput::FileUri`].
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub images: Vec<ImageInput>,
}

/// Start a background music generation. Returns the interaction handle to poll.
pub async fn generate_lyria_3_clip(args: GenerateLyria3ClipArgs<'_>) -> Result<InteractionHandle, GoogleGeminiError> {
  let GenerateLyria3ClipArgs { api_key, request } = args;
  if request.images.len() > 10 {
    return Err(
      crate::error::google_gemini_client_error::GoogleGeminiClientError::InvalidRequest(
        "Lyria accepts at most 10 inspiration images".to_string(),
      )
      .into(),
    );
  }
  let body = build_body(&request);
  run_background_interaction(api_key, &body).await
}

pub(crate) fn build_body(request: &Lyria3ClipRequest) -> InteractionRequestBody {
  let mut input = vec![InputItem::text(&request.prompt)];
  input.extend(request.images.iter().map(InputItem::image));
  InteractionRequestBody {
    system_instruction: None,
    tools: None,
    model: LYRIA_3_CLIP_MODEL_ID.to_string(),
    input,
    response_format: None,
    generation_config: None,
    previous_interaction_id: None,
    store: None,
    background: Some(true),
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  /// Input → exact outbound HTTP body.
  #[test]
  fn wire_shape() {
    let request = Lyria3ClipRequest {
      prompt: "lo-fi hip hop, 80 BPM, rainy-night mood, no vocals".to_string(),
      images: vec![],
    };
    assert_eq!(
      serde_json::to_value(build_body(&request)).unwrap(),
      json!({
        "model": "lyria-3-clip-preview",
        "input": [{ "type": "text", "text": "lo-fi hip hop, 80 BPM, rainy-night mood, no vocals" }],
        "background": true
      })
    );
  }

  /// Live call — costs real money ($0.04). Reads
  /// `external/credentials/gemini_api_key.txt`.
  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_generate_lyria_3_clip() -> anyhow::Result<()> {
    crate::test_utils::setup_test_logging::setup_test_logging();
    let api_key = crate::test_utils::get_test_api_key::get_test_api_key()?;
    let request = Lyria3ClipRequest {
      prompt: "A short, warm acoustic-guitar instrumental, gentle and optimistic".to_string(),
      images: vec![],
    };
    let handle = generate_lyria_3_clip(GenerateLyria3ClipArgs { api_key: &api_key, request }).await?;

    // Background: poll (5s cadence, ≤5 min) until terminal, then extract.
    use crate::api::requests::interactions::get_interaction::get_interaction::{
      get_interaction, GetInteractionArgs, InteractionPollOutcome,
    };
    for _ in 0..60 {
      tokio::time::sleep(std::time::Duration::from_secs(5)).await;
      match get_interaction(GetInteractionArgs { api_key: &api_key, interaction: &handle }).await? {
        InteractionPollOutcome::InProgress => continue,
        InteractionPollOutcome::Cancelled => anyhow::bail!("interaction cancelled"),
        InteractionPollOutcome::Failed { message } => anyhow::bail!("interaction failed: {message}"),
        InteractionPollOutcome::Completed(success) => {
          let music = music_success_from_interaction(success)?;
          assert!(!music.audio.is_empty(), "expected at least one audio item");
          assert!(music.audio[0].bytes.len() > 10_000, "suspiciously small audio");
          return Ok(());
        }
      }
    }
    anyhow::bail!("interaction did not complete within 5 minutes")
  }
}
