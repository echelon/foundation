use serde::Serialize;

use crate::api::requests::legacy_common::legacy_post;
use crate::api::types::element_types::element_image_list::ElementImageList;
use crate::api::types::element_types::element_tag_entry::ElementTagEntry;
use crate::api::types::element_types::element_video_list::ElementVideoList;
use crate::api::types::element_types::reference_type::ReferenceType;
use crate::api::types::legacy_types::legacy_created_task::LegacyCreatedTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/general/advanced-custom-elements";
const MAX_NAME_CHARS: usize = 20;
const MAX_DESCRIPTION_CHARS: usize = 100;

/// What the element is built from — exactly one of the two reference kinds,
/// so the required-list-per-type rule is unrepresentable to get wrong.
#[derive(Clone, Debug)]
pub enum ElementSource {
  /// A multi-image element: 1 frontal image + 1–3 reference images.
  /// Supported by every element-consuming endpoint.
  Images(ElementImageList),
  /// A video character element: exactly 1 reference video (`.mp4/.mov`,
  /// 3–8s, 1080P, 16:9 or 9:16, ≤200MB; realistic-style humanoids only).
  /// Video elements serve Kling 3.0 / 3.0 Omni video — NOT Kling O1. If the
  /// video contains a human voice, a custom voice is auto-created and bound.
  Video(ElementVideoList),
}

/// Create a custom element (async — poll with
/// [`crate::api::requests::elements::get_element`]). Element creation
/// pricing is not in Kling's published tables; the task's own
/// `final_unit_deduction`/`final_balance_deduction` are authoritative.
#[derive(Clone, Debug)]
pub struct CreateElementRequest {
  /// ≤20 chars.
  pub element_name: String,
  /// ≤100 chars.
  pub element_description: String,
  pub source: ElementSource,
  /// Bind an existing custom voice (multi-image elements: character/humanoid
  /// images only).
  pub element_voice_id: Option<String>,
  /// Category tags (see [`ElementTagEntry`] for the documented tag ids).
  pub tag_list: Vec<ElementTagEntry>,
  /// Legacy endpoints take callback/correlation ids at the top level (not in
  /// an `options` object). Callbacks use the legacy dialect →
  /// [`crate::api::webhooks::parse_legacy_callback`].
  pub callback_url: Option<String>,
  pub external_task_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CreateElementArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: CreateElementRequest,
}

#[derive(Serialize)]
pub(crate) struct CreateElementBody<'a> {
  element_name: &'a str,
  element_description: &'a str,
  reference_type: ReferenceType,
  #[serde(skip_serializing_if = "Option::is_none")]
  element_image_list: Option<&'a ElementImageList>,
  #[serde(skip_serializing_if = "Option::is_none")]
  element_video_list: Option<&'a ElementVideoList>,
  #[serde(skip_serializing_if = "Option::is_none")]
  element_voice_id: Option<&'a str>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  tag_list: Vec<&'a ElementTagEntry>,
  #[serde(skip_serializing_if = "Option::is_none")]
  callback_url: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  external_task_id: Option<&'a str>,
}

pub(crate) fn build_body(request: &CreateElementRequest) -> Result<CreateElementBody<'_>, KlingClientError> {
  let name_chars = request.element_name.chars().count();
  if request.element_name.trim().is_empty() || name_chars > MAX_NAME_CHARS {
    return Err(KlingClientError::InvalidRequest(format!(
      "element_name must be 1..={MAX_NAME_CHARS} chars, got {name_chars}"
    )));
  }
  let description_chars = request.element_description.chars().count();
  if request.element_description.trim().is_empty() || description_chars > MAX_DESCRIPTION_CHARS {
    return Err(KlingClientError::InvalidRequest(format!(
      "element_description must be 1..={MAX_DESCRIPTION_CHARS} chars, got {description_chars}"
    )));
  }

  let (reference_type, element_image_list, element_video_list) = match &request.source {
    ElementSource::Images(images) => {
      if images.frontal_image.as_deref().unwrap_or("").trim().is_empty() {
        return Err(KlingClientError::InvalidRequest("an image element requires a frontal_image".into()));
      }
      if images.refer_images.is_empty() || images.refer_images.len() > 3 {
        return Err(KlingClientError::InvalidRequest(format!(
          "an image element requires 1..=3 refer_images, got {}",
          images.refer_images.len()
        )));
      }
      (ReferenceType::ImageRefer, Some(images), None)
    }
    ElementSource::Video(videos) => {
      if videos.refer_videos.len() != 1 {
        return Err(KlingClientError::InvalidRequest(format!(
          "a video element requires exactly 1 refer_video, got {}",
          videos.refer_videos.len()
        )));
      }
      (ReferenceType::VideoRefer, None, Some(videos))
    }
  };

  Ok(CreateElementBody {
    element_name: &request.element_name,
    element_description: &request.element_description,
    reference_type,
    element_image_list,
    element_video_list,
    element_voice_id: request.element_voice_id.as_deref(),
    tag_list: request.tag_list.iter().collect(),
    callback_url: request.callback_url.as_deref(),
    external_task_id: request.external_task_id.as_deref(),
  })
}

/// Dry-run: the exact JSON body [`create_element`] would send.
pub fn create_element_wire_json(request: &CreateElementRequest) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /v1/general/advanced-custom-elements` — async element creation;
/// poll with [`crate::api::requests::elements::get_element`].
pub async fn create_element(args: CreateElementArgs<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let CreateElementArgs { api_key, request } = args;
  let body = build_body(&request)?;
  let body_json = serde_json::to_string(&body).map_err(KlingClientError::RequestSerializationError)?;
  legacy_post(api_key, PATH, body_json).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;
  use crate::api::types::element_types::element_image_list::ReferImage;
  use crate::api::types::element_types::element_video_list::ReferVideo;

  fn image_source() -> ElementSource {
    ElementSource::Images(ElementImageList {
      frontal_image: Some("https://example.com/frontal.png".to_string()),
      refer_images: vec![ReferImage { image_url: "https://example.com/side.png".to_string() }],
    })
  }

  // ── Wire-shape tests ──

  #[test]
  fn wire_shape_image_element() {
    let request = CreateElementRequest {
      element_name: "Zhang".to_string(),
      element_description: "protagonist in a red coat".to_string(),
      source: image_source(),
      element_voice_id: Some("850087542757535834".to_string()),
      tag_list: vec![ElementTagEntry::from_tag_id("o_102")],
      callback_url: Some("https://example.com/webhooks/kling".to_string()),
      external_task_id: Some("job_01ABC".to_string()),
    };
    assert_eq!(
      create_element_wire_json(&request).unwrap(),
      json!({
        "element_name": "Zhang",
        "element_description": "protagonist in a red coat",
        "reference_type": "image_refer",
        "element_image_list": {
          "frontal_image": "https://example.com/frontal.png",
          "refer_images": [{"image_url": "https://example.com/side.png"}]
        },
        "element_voice_id": "850087542757535834",
        "tag_list": [{"tag_id": "o_102"}],
        "callback_url": "https://example.com/webhooks/kling",
        "external_task_id": "job_01ABC"
      })
    );
  }

  #[test]
  fn wire_shape_video_element() {
    let request = CreateElementRequest {
      element_name: "Zhang".to_string(),
      element_description: "protagonist".to_string(),
      source: ElementSource::Video(ElementVideoList {
        refer_videos: vec![ReferVideo { video_url: "https://example.com/ref.mp4".to_string() }],
      }),
      element_voice_id: None,
      tag_list: vec![],
      callback_url: None,
      external_task_id: None,
    };
    assert_eq!(
      create_element_wire_json(&request).unwrap(),
      json!({
        "element_name": "Zhang",
        "element_description": "protagonist",
        "reference_type": "video_refer",
        "element_video_list": {
          "refer_videos": [{"video_url": "https://example.com/ref.mp4"}]
        }
      })
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_overlong_name_and_description() {
    let mut request = CreateElementRequest {
      element_name: "x".repeat(21),
      element_description: "d".to_string(),
      source: image_source(),
      element_voice_id: None,
      tag_list: vec![],
      callback_url: None,
      external_task_id: None,
    };
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));

    request.element_name = "ok".to_string();
    request.element_description = "x".repeat(101);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn image_element_requires_frontal_and_1_to_3_refs() {
    let no_frontal = CreateElementRequest {
      element_name: "e".to_string(),
      element_description: "d".to_string(),
      source: ElementSource::Images(ElementImageList {
        frontal_image: None,
        refer_images: vec![ReferImage { image_url: "https://example.com/s.png".to_string() }],
      }),
      element_voice_id: None,
      tag_list: vec![],
      callback_url: None,
      external_task_id: None,
    };
    assert!(matches!(build_body(&no_frontal), Err(KlingClientError::InvalidRequest(_))));

    let too_many_refs = CreateElementRequest {
      element_name: "e".to_string(),
      element_description: "d".to_string(),
      source: ElementSource::Images(ElementImageList {
        frontal_image: Some("https://example.com/f.png".to_string()),
        refer_images: (0..4)
          .map(|i| ReferImage { image_url: format!("https://example.com/{i}.png") })
          .collect(),
      }),
      element_voice_id: None,
      tag_list: vec![],
      callback_url: None,
      external_task_id: None,
    };
    assert!(matches!(build_body(&too_many_refs), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn video_element_requires_exactly_one_video() {
    let request = CreateElementRequest {
      element_name: "e".to_string(),
      element_description: "d".to_string(),
      source: ElementSource::Video(ElementVideoList { refer_videos: vec![] }),
      element_voice_id: None,
      tag_list: vec![],
      callback_url: None,
      external_task_id: None,
    };
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
