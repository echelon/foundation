use serde::Serialize;

use crate::api::requests::legacy_common::legacy_post;
use crate::api::types::legacy_types::legacy_created_task::LegacyCreatedTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/audio/text-to-audio";
const MAX_PROMPT_CHARS: usize = 200;
const MIN_DURATION_SECONDS: f64 = 3.0;
const MAX_DURATION_SECONDS: f64 = 10.0;

/// Kling text-to-audio: generate a sound effect / ambience clip from a short
/// description. Results come back in both MP3 and WAV.
#[derive(Clone, Debug)]
pub struct TextToAudioRequest {
  /// ≤200 chars.
  pub prompt: String,
  /// 3.0–10.0 seconds, one decimal place.
  pub duration: f64,
  /// Callbacks use the legacy dialect →
  /// [`crate::api::webhooks::parse_legacy_callback`].
  pub callback_url: Option<String>,
  pub external_task_id: Option<String>,
}

impl TextToAudioRequest {
  pub fn new(prompt: impl Into<String>, duration: f64) -> Self {
    Self { prompt: prompt.into(), duration, callback_url: None, external_task_id: None }
  }
}

#[derive(Clone, Debug)]
pub struct TextToAudioArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: TextToAudioRequest,
}

#[derive(Serialize)]
pub(crate) struct TextToAudioBody<'a> {
  prompt: &'a str,
  duration: f64,
  #[serde(skip_serializing_if = "Option::is_none")]
  callback_url: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  external_task_id: Option<&'a str>,
}

pub(crate) fn build_body(request: &TextToAudioRequest) -> Result<TextToAudioBody<'_>, KlingClientError> {
  if request.prompt.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("prompt must not be empty".into()));
  }
  let chars = request.prompt.chars().count();
  if chars > MAX_PROMPT_CHARS {
    return Err(KlingClientError::InvalidRequest(format!(
      "prompt is {chars} chars; text-to-audio allows at most {MAX_PROMPT_CHARS}"
    )));
  }
  if !(MIN_DURATION_SECONDS..=MAX_DURATION_SECONDS).contains(&request.duration) {
    return Err(KlingClientError::InvalidRequest(format!(
      "duration must be in {MIN_DURATION_SECONDS}..={MAX_DURATION_SECONDS} seconds, got {}",
      request.duration
    )));
  }
  // The API takes one decimal place; round rather than reject.
  let duration = (request.duration * 10.0).round() / 10.0;
  Ok(TextToAudioBody {
    prompt: &request.prompt,
    duration,
    callback_url: request.callback_url.as_deref(),
    external_task_id: request.external_task_id.as_deref(),
  })
}

/// Dry-run: the exact JSON body [`text_to_audio`] would send.
pub fn text_to_audio_wire_json(request: &TextToAudioRequest) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /v1/audio/text-to-audio` — async; poll with
/// [`crate::api::requests::audio::get_text_to_audio_task`].
pub async fn text_to_audio(args: TextToAudioArgs<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let TextToAudioArgs { api_key, request } = args;
  let body = build_body(&request)?;
  let body_json = serde_json::to_string(&body).map_err(KlingClientError::RequestSerializationError)?;
  legacy_post(api_key, PATH, body_json).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  #[test]
  fn wire_shape_full_request() {
    let mut request = TextToAudioRequest::new("rain on a tin roof, distant thunder", 6.5);
    request.callback_url = Some("https://example.com/webhooks/kling".to_string());
    request.external_task_id = Some("job_01ABC".to_string());
    assert_eq!(
      text_to_audio_wire_json(&request).unwrap(),
      json!({
        "prompt": "rain on a tin roof, distant thunder",
        "duration": 6.5,
        "callback_url": "https://example.com/webhooks/kling",
        "external_task_id": "job_01ABC"
      })
    );
  }

  #[test]
  fn duration_is_rounded_to_one_decimal() {
    let request = TextToAudioRequest::new("wind", 5.55);
    let json = text_to_audio_wire_json(&request).unwrap();
    assert_eq!(json["duration"], json!(5.6));
  }

  // ── Validation tests ──

  #[test]
  fn rejects_out_of_range_duration_and_long_prompt() {
    assert!(matches!(
      build_body(&TextToAudioRequest::new("wind", 2.9)),
      Err(KlingClientError::InvalidRequest(_))
    ));
    assert!(matches!(
      build_body(&TextToAudioRequest::new("wind", 10.1)),
      Err(KlingClientError::InvalidRequest(_))
    ));
    assert!(matches!(
      build_body(&TextToAudioRequest::new("x".repeat(201), 5.0)),
      Err(KlingClientError::InvalidRequest(_))
    ));
  }
}
