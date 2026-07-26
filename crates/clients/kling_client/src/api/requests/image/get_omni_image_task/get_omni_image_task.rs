use crate::api::requests::image::get_image_task::get_image_task::ImageTaskRef;
use crate::api::requests::legacy_common::legacy_get;
use crate::api::types::image_types::image_task_result::ImageTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

#[derive(Clone, Debug)]
pub struct GetOmniImageTaskArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub task: ImageTaskRef,
}

/// `GET /v1/images/omni-image/{id}` — poll a Kling Image 3.0 Omni / Image O1
/// generation (legacy vocabulary; status `succeed`). Single results land in
/// `task_result.images`; series results in `task_result.series_images` with
/// `result_type: "series"`.
pub async fn get_omni_image_task(args: GetOmniImageTaskArgs<'_>) -> Result<LegacyTask<ImageTaskResult>, KlingError> {
  let GetOmniImageTaskArgs { api_key, task } = args;
  let id = task.as_path_segment();
  if id.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("omni image task id must not be empty".into()).into());
  }
  let path = format!("/v1/images/omni-image/{id}");
  legacy_get(api_key, &path, &[]).await
}

#[cfg(test)]
mod tests {
  use crate::api::types::image_types::image_task_result::ImageTaskResult;
  use crate::api::types::legacy_types::legacy_task::LegacyTask;

  #[test]
  fn series_result_parses() {
    let json = r#"{
      "task_id": "1",
      "task_status": "succeed",
      "task_result": {
        "result_type": "series",
        "series_images": [
          {"index": 0, "url": "https://cdn.example/s0.png"},
          {"index": 1, "url": "https://cdn.example/s1.png"}
        ]
      }
    }"#;
    let task: LegacyTask<ImageTaskResult> = serde_json::from_str(json).unwrap();
    let result = task.task_result.unwrap();
    assert_eq!(result.result_type.as_deref(), Some("series"));
    assert_eq!(result.series_images.len(), 2);
    assert!(result.images.is_empty());
  }
}
