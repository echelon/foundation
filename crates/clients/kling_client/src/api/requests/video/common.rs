use client_core::http_client;
use serde::Serialize;
use tracing::info;

use crate::api::requests::kling_host::{with_auth, KLING_API_BASE_URL, KLING_USER_AGENT};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::kling_envelope::KlingEnvelope;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::classify_kling_http_error::classify_kling_http_error;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;
use crate::error::kling_generic_api_error::KlingGenericApiError;

/// The outbound body shared by every new-style video create endpoint.
/// Text-to-video uses the top-level `prompt`; every other endpoint expresses
/// its inputs as a `contents` array.
#[derive(Debug, Serialize)]
pub(crate) struct VideoCreateBody<'a> {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub prompt: Option<&'a str>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub contents: Vec<ContentItemWire<'a>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub settings: Option<SettingsWire>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub options: Option<&'a RequestOptions>,
}

/// One entry of a create request's `contents` array — tagged on `type`.
/// Image/frame URLs accept both hosted URLs and raw Base64; reference-video
/// URLs must be hosted URLs.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum ContentItemWire<'a> {
  #[serde(rename = "prompt")]
  Prompt { text: &'a str },
  #[serde(rename = "first_frame")]
  FirstFrame { url: &'a str },
  #[serde(rename = "last_frame")]
  LastFrame { url: &'a str },
  #[serde(rename = "refer_image")]
  ReferImage {
    url: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
  },
  #[serde(rename = "feature_video")]
  FeatureVideo {
    url: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
  },
  #[serde(rename = "base_video")]
  BaseVideo {
    url: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
  },
  /// Motion control: the character appearance reference image.
  #[serde(rename = "image")]
  Image { url: &'a str },
  /// Motion control: the motion reference video.
  #[serde(rename = "video")]
  Video { url: &'a str },
  #[serde(rename = "element")]
  Element { element_id: &'a str, id: &'a str },
  #[serde(rename = "voice")]
  Voice { voice_id: &'a str, id: &'a str },
}

/// The `settings` object — a superset across endpoints; each binding fills
/// only the fields its endpoint documents.
#[derive(Debug, Default, Serialize)]
pub(crate) struct SettingsWire {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub multi_shot: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub audio: Option<&'static str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolution: Option<&'static str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<&'static str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub duration: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub character_orientation: Option<&'static str>,
}

impl SettingsWire {
  pub(crate) fn into_option(self) -> Option<Self> {
    let is_empty = self.multi_shot.is_none()
      && self.audio.is_none()
      && self.resolution.is_none()
      && self.aspect_ratio.is_none()
      && self.duration.is_none()
      && self.character_orientation.is_none();
    if is_empty {
      None
    } else {
      Some(self)
    }
  }
}

/// Wrap [`RequestOptions`] for the wire, omitting it entirely when empty.
pub(crate) fn options_wire(options: &RequestOptions) -> Option<&RequestOptions> {
  if options.is_empty() {
    None
  } else {
    Some(options)
  }
}

pub(crate) struct CreateVideoTaskParts<'a> {
  pub api_key: &'a KlingApiKey,
  /// Endpoint path, e.g. `/text-to-video/kling-3.0-turbo`.
  pub path: &'static str,
  pub body: &'a VideoCreateBody<'a>,
}

/// POST a create body to a new-style video endpoint and return the task
/// handle. Poll with [`crate::api::requests::tasks`].
pub(crate) async fn create_video_task(parts: CreateVideoTaskParts<'_>) -> Result<CreatedTask, KlingError> {
  let CreateVideoTaskParts { api_key, path, body } = parts;

  // Pre-serialize so a serialization failure is reported as a client error
  // (not swallowed inside reqwest's send path).
  let body_json = serde_json::to_string(body).map_err(KlingClientError::RequestSerializationError)?;

  info!(path, "Kling create video task");

  let url = format!("{KLING_API_BASE_URL}{path}");
  let client = http_client(KLING_USER_AGENT);

  let response = with_auth(client.post(&url), api_key)
    .header("Content-Type", "application/json")
    .body(body_json)
    .send()
    .await
    .map_err(KlingGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(KlingGenericApiError::ReqwestError)?;

  info!(status = %status, path, "Kling create video task response");

  classify_kling_http_error(status, Some(&response_body))?;

  let parsed: KlingEnvelope<CreatedTask> = serde_json::from_str(&response_body)
    .map_err(|e| KlingGenericApiError::SerdeResponseParseErrorWithBody(e, response_body.clone()))?;

  parsed.into_data(&response_body)
}

/// Shared prompt validation: required prompts must be non-empty; all prompts
/// respect the endpoint's character ceiling.
pub(crate) fn validate_prompt(prompt: &str, max_chars: usize) -> Result<(), KlingClientError> {
  if prompt.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("prompt must not be empty".into()));
  }
  let chars = prompt.chars().count();
  if chars > max_chars {
    return Err(KlingClientError::InvalidRequest(format!(
      "prompt is {chars} chars; this endpoint allows at most {max_chars}"
    )));
  }
  Ok(())
}

/// Shared duration validation for the 3.0-family endpoints that accept any
/// integer seconds within a range.
pub(crate) fn validate_duration_range(duration: u32, min: u32, max: u32) -> Result<(), KlingClientError> {
  if duration < min || duration > max {
    return Err(KlingClientError::InvalidRequest(format!(
      "duration must be in {min}..={max} seconds, got {duration}"
    )));
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn contents_items_serialize_with_type_tags() {
    let contents = vec![
      ContentItemWire::Prompt { text: "a fox" },
      ContentItemWire::FirstFrame { url: "https://example.com/f.png" },
      ContentItemWire::Element { element_id: "829836802793406551", id: "Zhang" },
    ];
    assert_eq!(
      serde_json::to_value(&contents).unwrap(),
      json!([
        {"type": "prompt", "text": "a fox"},
        {"type": "first_frame", "url": "https://example.com/f.png"},
        {"type": "element", "element_id": "829836802793406551", "id": "Zhang"}
      ])
    );
  }

  #[test]
  fn refer_image_omits_absent_id() {
    let item = ContentItemWire::ReferImage { url: "https://example.com/r.png", id: None };
    assert_eq!(
      serde_json::to_value(&item).unwrap(),
      json!({"type": "refer_image", "url": "https://example.com/r.png"})
    );
  }

  #[test]
  fn empty_settings_collapse_to_none() {
    assert!(SettingsWire::default().into_option().is_none());
    let settings = SettingsWire { duration: Some(5), ..Default::default() };
    assert!(settings.into_option().is_some());
  }

  #[test]
  fn prompt_validation() {
    assert!(validate_prompt("a fox", 2500).is_ok());
    assert!(matches!(validate_prompt("  ", 2500), Err(KlingClientError::InvalidRequest(_))));
    assert!(matches!(validate_prompt(&"x".repeat(2501), 2500), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn duration_validation() {
    assert!(validate_duration_range(3, 3, 15).is_ok());
    assert!(validate_duration_range(15, 3, 15).is_ok());
    assert!(matches!(validate_duration_range(2, 3, 15), Err(KlingClientError::InvalidRequest(_))));
    assert!(matches!(validate_duration_range(16, 3, 15), Err(KlingClientError::InvalidRequest(_))));
  }
}
