use crate::api::requests::legacy_common::legacy_get;
use crate::api::types::image_types::image_task_result::ImageTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

/// Which id to look up — the system `task_id`, or the caller-chosen
/// `external_task_id`. Both fill the same path slot.
#[derive(Clone, Debug)]
pub enum ImageTaskRef {
  TaskId(String),
  ExternalTaskId(String),
}

impl ImageTaskRef {
  pub(crate) fn as_path_segment(&self) -> &str {
    match self {
      Self::TaskId(id) | Self::ExternalTaskId(id) => id,
    }
  }
}

#[derive(Clone, Debug)]
pub struct GetImageTaskArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub task: ImageTaskRef,
}

/// `GET /v1/images/generations/{id}` — poll a Kling Image 3.0 generation
/// (legacy vocabulary; status `succeed`). Generated images appear in
/// `task_result.images` once terminal.
pub async fn get_image_task(args: GetImageTaskArgs<'_>) -> Result<LegacyTask<ImageTaskResult>, KlingError> {
  let GetImageTaskArgs { api_key, task } = args;
  let id = task.as_path_segment();
  if id.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("image task id must not be empty".into()).into());
  }
  let path = format!("/v1/images/generations/{id}");
  legacy_get(api_key, &path, &[]).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::legacy_types::legacy_task_status::LegacyTaskStatus;

  /// The documented query response parses end to end.
  #[test]
  fn documented_response_parses() {
    let json = r#"{
      "task_id": "738552951910035532",
      "task_status": "succeed",
      "task_status_msg": "",
      "task_info": {"external_task_id": "custom_1"},
      "task_result": {
        "images": [
          {"index": 0, "url": "https://cdn.example/0.png", "watermark_url": "https://cdn.example/0w.png"},
          {"index": 1, "url": "https://cdn.example/1.png", "watermark_url": "https://cdn.example/1w.png"}
        ]
      },
      "final_unit_deduction": "16",
      "final_balance_deduction": {"quota": "0.056", "list_price": "0.056"},
      "created_at": 1722769557708,
      "updated_at": 1722769557708
    }"#;
    let task: LegacyTask<ImageTaskResult> = serde_json::from_str(json).unwrap();
    assert_eq!(task.task_status, LegacyTaskStatus::Succeeded);
    let result = task.task_result.unwrap();
    assert_eq!(result.images.len(), 2);
    assert_eq!(result.images[1].index, Some(1));
    assert_eq!(task.final_unit_deduction.as_deref(), Some("16"));
  }
}
