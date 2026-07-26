use crate::api::requests::legacy_common::legacy_get;
use crate::api::types::element_types::element_task_result::ElementTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

/// Which id to look up — the system `task_id` from
/// [`crate::api::requests::elements::create_element`], or the caller-chosen
/// `external_task_id`. Both fill the same path slot.
#[derive(Clone, Debug)]
pub enum ElementTaskRef {
  TaskId(String),
  ExternalTaskId(String),
}

impl ElementTaskRef {
  pub(crate) fn as_path_segment(&self) -> &str {
    match self {
      Self::TaskId(id) | Self::ExternalTaskId(id) => id,
    }
  }
}

#[derive(Clone, Debug)]
pub struct GetElementArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub task: ElementTaskRef,
}

/// `GET /v1/general/advanced-custom-elements/{id}` — fetch one element
/// creation task (legacy vocabulary; status `succeed`). The created
/// element(s) appear in `task_result.elements` once terminal.
pub async fn get_element(args: GetElementArgs<'_>) -> Result<LegacyTask<ElementTaskResult>, KlingError> {
  let GetElementArgs { api_key, task } = args;
  let id = task.as_path_segment();
  if id.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("element task id must not be empty".into()).into());
  }
  let path = format!("/v1/general/advanced-custom-elements/{id}");
  legacy_get(api_key, &path, &[]).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::legacy_types::legacy_task_status::LegacyTaskStatus;

  /// The documented single-element query response parses end to end.
  #[test]
  fn documented_response_parses() {
    let json = r#"{
      "task_id": "738552951910035532",
      "task_status": "succeed",
      "task_status_msg": "",
      "task_info": {"external_task_id": "custom_1"},
      "task_result": {
        "elements": [{
          "element_id": 829836802793406551,
          "element_name": "Zhang",
          "element_description": "protagonist",
          "reference_type": "video_refer",
          "element_image_list": {},
          "element_video_list": {},
          "element_voice_info": {"voice_id": "v_1", "voice_name": "Zhang voice", "trial_url": "https://cdn.example/t.mp3", "owned_by": "kling"},
          "tag_list": [],
          "owned_by": "kling",
          "status": "succeed"
        }]
      },
      "final_unit_deduction": "0",
      "final_balance_deduction": {"quota": "0", "list_price": "0"},
      "created_at": 1722769557708,
      "updated_at": 1722769557708
    }"#;
    let task: LegacyTask<ElementTaskResult> = serde_json::from_str(json).unwrap();
    assert_eq!(task.task_status, LegacyTaskStatus::Succeeded);
    let elements = task.task_result.unwrap().elements;
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].element_voice_info.as_ref().unwrap().voice_id.as_deref(), Some("v_1"));
  }
}
