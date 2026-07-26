use serde::Serialize;

use crate::api::requests::legacy_common::legacy_post;
use crate::api::types::legacy_types::legacy_created_task::LegacyCreatedTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/general/custom-voices";
const MAX_NAME_CHARS: usize = 20;

/// Where the voice sample comes from — one of the two documented sources
/// (the docs strongly imply they're mutually exclusive; the enum enforces
/// that).
#[derive(Clone, Debug)]
pub enum VoiceSource {
  /// `.mp3/.wav` audio or `.mp4/.mov` video URL — a clean single voice,
  /// 5–30s.
  Url(String),
  /// A previously generated Kling video id: a V2.6 generation with sound on,
  /// or an Avatar / Lip-Sync API output.
  VideoId(String),
}

/// Create a custom voice (async — poll with
/// [`crate::api::requests::voices::get_voice`]).
#[derive(Clone, Debug)]
pub struct CreateVoiceRequest {
  /// ≤20 chars.
  pub voice_name: String,
  pub source: VoiceSource,
  /// Legacy endpoints take callback/correlation ids at the top level (not in
  /// an `options` object). Callbacks use the legacy dialect →
  /// [`crate::api::webhooks::parse_legacy_callback`].
  pub callback_url: Option<String>,
  pub external_task_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CreateVoiceArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: CreateVoiceRequest,
}

#[derive(Serialize)]
pub(crate) struct CreateVoiceBody<'a> {
  voice_name: &'a str,
  #[serde(skip_serializing_if = "Option::is_none")]
  voice_url: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  video_id: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  callback_url: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  external_task_id: Option<&'a str>,
}

pub(crate) fn build_body(request: &CreateVoiceRequest) -> Result<CreateVoiceBody<'_>, KlingClientError> {
  let name_chars = request.voice_name.chars().count();
  if request.voice_name.trim().is_empty() || name_chars > MAX_NAME_CHARS {
    return Err(KlingClientError::InvalidRequest(format!(
      "voice_name must be 1..={MAX_NAME_CHARS} chars, got {name_chars}"
    )));
  }
  let (voice_url, video_id) = match &request.source {
    VoiceSource::Url(url) => {
      if url.trim().is_empty() {
        return Err(KlingClientError::InvalidRequest("voice_url must not be empty".into()));
      }
      (Some(url.as_str()), None)
    }
    VoiceSource::VideoId(id) => {
      if id.trim().is_empty() {
        return Err(KlingClientError::InvalidRequest("video_id must not be empty".into()));
      }
      (None, Some(id.as_str()))
    }
  };
  Ok(CreateVoiceBody {
    voice_name: &request.voice_name,
    voice_url,
    video_id,
    callback_url: request.callback_url.as_deref(),
    external_task_id: request.external_task_id.as_deref(),
  })
}

/// Dry-run: the exact JSON body [`create_voice`] would send.
pub fn create_voice_wire_json(request: &CreateVoiceRequest) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /v1/general/custom-voices` — async voice creation; poll with
/// [`crate::api::requests::voices::get_voice`].
pub async fn create_voice(args: CreateVoiceArgs<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let CreateVoiceArgs { api_key, request } = args;
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
  fn wire_shape_from_url() {
    let request = CreateVoiceRequest {
      voice_name: "Narrator".to_string(),
      source: VoiceSource::Url("https://example.com/sample.mp3".to_string()),
      callback_url: Some("https://example.com/webhooks/kling".to_string()),
      external_task_id: Some("job_01ABC".to_string()),
    };
    assert_eq!(
      create_voice_wire_json(&request).unwrap(),
      json!({
        "voice_name": "Narrator",
        "voice_url": "https://example.com/sample.mp3",
        "callback_url": "https://example.com/webhooks/kling",
        "external_task_id": "job_01ABC"
      })
    );
  }

  #[test]
  fn wire_shape_from_video_id_never_emits_both_sources() {
    let request = CreateVoiceRequest {
      voice_name: "Narrator".to_string(),
      source: VoiceSource::VideoId("893605946402811985".to_string()),
      callback_url: None,
      external_task_id: None,
    };
    let json = create_voice_wire_json(&request).unwrap();
    assert_eq!(
      json,
      json!({"voice_name": "Narrator", "video_id": "893605946402811985"})
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_bad_names_and_empty_sources() {
    let overlong = CreateVoiceRequest {
      voice_name: "x".repeat(21),
      source: VoiceSource::Url("https://example.com/s.mp3".to_string()),
      callback_url: None,
      external_task_id: None,
    };
    assert!(matches!(build_body(&overlong), Err(KlingClientError::InvalidRequest(_))));

    let empty_url = CreateVoiceRequest {
      voice_name: "ok".to_string(),
      source: VoiceSource::Url("  ".to_string()),
      callback_url: None,
      external_task_id: None,
    };
    assert!(matches!(build_body(&empty_url), Err(KlingClientError::InvalidRequest(_))));
  }
}
