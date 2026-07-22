//! `POST /v1beta/interactions` — **Lyria 3 Pro** (`lyria-3-pro-preview`), type `music`.
//! Full-length songs with verses/choruses/bridges — a couple of minutes, steered by the prompt. 44.1 kHz stereo; MP3 by default. Musical controls (BPM, key,
//! duration, structure) are steered through natural language in the prompt —
//! the API exposes no discrete knobs for them. Synchronous — run from a worker.
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/music-generation>

use serde::Serialize;

use crate::api::requests::interactions::common::{run_interaction, InteractionRequestBody, InputItem};
use crate::api::requests::interactions::common::ResponseFormatWire;
use crate::api::requests::music::common::music_success_from_interaction;
use crate::api::types::image_types::image_input::ImageInput;
use crate::creds::google_aistudio_api_key::GoogleAiStudioApiKey;
use crate::error::google_aistudio_error::GoogleAiStudioError;

pub use crate::api::requests::music::common::{GeneratedAudio, GenerateMusicSuccess};

/// The exact model identifier this binding pins.
pub const LYRIA_3_PRO_MODEL_ID: &str = "lyria-3-pro-preview";

/// Top-level argument to [`generate_lyria_3_pro`]. Borrows the API key separately
/// from the request body so callers can log/persist (and PRICE) the request
/// without touching the credential.
#[derive(Clone, Debug)]
pub struct GenerateLyria3ProArgs<'a> {
  pub api_key: &'a GoogleAiStudioApiKey,
  pub request: Lyria3ProRequest,
}

/// The material part of a Lyria 3 Pro request — drives both the wire body and the
/// [`client_core::CostCalculator`] price (see `cost.rs`).
#[derive(Clone, Debug, Serialize)]
pub struct Lyria3ProRequest {
  /// The music prompt — genre, mood, instrumentation, structure, BPM, key,
  /// lyrics direction. The more specific, the better.
  pub prompt: String,

  /// Optional inspiration images (≤ 10). Prefer [`ImageInput::FileUri`].
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub images: Vec<ImageInput>,

  /// Request lossless WAV output instead of the MP3 default (Pro only).
  pub wav_output: bool,
}

/// Generate music with Lyria 3 Pro. Returns decoded audio (plus lyrics when the
/// model wrote them).
pub async fn generate_lyria_3_pro(args: GenerateLyria3ProArgs<'_>) -> Result<GenerateMusicSuccess, GoogleAiStudioError> {
  let GenerateLyria3ProArgs { api_key, request } = args;
  let body = build_body(&request);
  let interaction = run_interaction(api_key, &body).await?;
  music_success_from_interaction(interaction)
}

pub(crate) fn build_body(request: &Lyria3ProRequest) -> InteractionRequestBody {
  let mut input = vec![InputItem::text(&request.prompt)];
  input.extend(request.images.iter().map(InputItem::image));
  InteractionRequestBody {
    model: LYRIA_3_PRO_MODEL_ID.to_string(),
    input,
    response_format: if request.wav_output {
      Some(ResponseFormatWire { kind: "audio", aspect_ratio: None, delivery: None })
    } else {
      None
    },
    generation_config: None,
    previous_interaction_id: None,
    store: None,
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  /// Input → exact outbound HTTP body.
  #[test]
  fn wire_shape() {
    let request = Lyria3ProRequest {
      prompt: "lo-fi hip hop, 80 BPM, rainy-night mood, no vocals".to_string(),
      images: vec![],
      wav_output: false,
    };
    assert_eq!(
      serde_json::to_value(build_body(&request)).unwrap(),
      json!({
        "model": "lyria-3-pro-preview",
        "input": [{ "type": "text", "text": "lo-fi hip hop, 80 BPM, rainy-night mood, no vocals" }]
      })
    );
  }

  /// Live call — costs real money ($0.08). Reads
  /// `external/credentials/google_aistudio_api_key.txt`.
  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_generate_lyria_3_pro() -> anyhow::Result<()> {
    crate::test_utils::setup_test_logging::setup_test_logging();
    let api_key = crate::test_utils::get_test_api_key::get_test_api_key()?;
    let request = Lyria3ProRequest {
      prompt: "A short, warm acoustic-guitar instrumental, gentle and optimistic".to_string(),
      images: vec![],
      wav_output: false,
    };
    let success = generate_lyria_3_pro(GenerateLyria3ProArgs { api_key: &api_key, request }).await?;
    assert!(!success.audio.is_empty(), "expected at least one audio item");
    assert!(success.audio[0].bytes.len() > 10_000, "suspiciously small audio");
    Ok(())
  }
}
