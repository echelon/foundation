use serde::Deserialize;

use crate::api::types::element_types::element_id::ElementId;
use crate::api::types::element_types::element_image_list::ElementImageList;
use crate::api::types::element_types::element_tag_entry::ElementTagEntry;
use crate::api::types::element_types::element_video_list::ElementVideoList;
use crate::api::types::element_types::element_voice_info::ElementVoiceInfo;
use crate::api::types::element_types::reference_type::ReferenceType;

/// A custom (or preset) element as returned by the element management
/// endpoints' `task_result.elements` arrays.
#[derive(Clone, Debug, Deserialize)]
pub struct ElementRecord {
  pub element_id: ElementId,
  #[serde(default)]
  pub element_name: Option<String>,
  #[serde(default)]
  pub element_description: Option<String>,
  #[serde(default)]
  pub reference_type: Option<ReferenceType>,
  #[serde(default)]
  pub element_image_list: Option<ElementImageList>,
  #[serde(default)]
  pub element_video_list: Option<ElementVideoList>,
  #[serde(default)]
  pub element_voice_info: Option<ElementVoiceInfo>,
  #[serde(default)]
  pub tag_list: Vec<ElementTagEntry>,
  /// `"kling"` for the official library, otherwise a creator id.
  #[serde(default)]
  pub owned_by: Option<String>,
  /// `succeed` when live, `deleted` when removed.
  #[serde(default)]
  pub status: Option<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn documented_element_record_parses_with_numeric_id() {
    let json = r#"{
      "element_id": 829836802793406551,
      "element_name": "Zhang",
      "element_description": "protagonist",
      "reference_type": "video_refer",
      "element_image_list": {},
      "element_video_list": {"refer_videos": [{"video_url": "https://cdn.example/ref.mp4"}]},
      "element_voice_info": {"voice_id": "v_1", "voice_name": "Zhang voice", "trial_url": "https://cdn.example/t.mp3", "owned_by": "kling"},
      "tag_list": [],
      "owned_by": "kling",
      "status": "succeed"
    }"#;
    let record: ElementRecord = serde_json::from_str(json).unwrap();
    assert_eq!(record.element_id.as_string(), "829836802793406551");
    assert_eq!(record.reference_type, Some(ReferenceType::VideoRefer));
    assert_eq!(record.element_video_list.unwrap().refer_videos.len(), 1);
  }
}
