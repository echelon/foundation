use serde::Serialize;

use crate::api::requests::legacy_common::legacy_post;
use crate::api::types::legacy_types::legacy_created_task::LegacyCreatedTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

/// The video-doc spelling of the delete path. The image element docs spell
/// the same operation `/v1/general/delete-elements` (identical body and
/// response — Kling doc drift); this binding uses the `advanced-` spelling
/// to match the `advanced-custom-elements` family it deletes from.
pub(crate) const PATH: &str = "/v1/general/delete-advanced-elements";

#[derive(Clone, Debug)]
pub struct DeleteElementArgs<'a> {
  pub api_key: &'a KlingApiKey,
  /// Custom elements only — presets can't be deleted. Sent as a string (the
  /// wire type the delete doc specifies, even though queries return numbers).
  pub element_id: String,
}

#[derive(Serialize)]
pub(crate) struct DeleteElementBody<'a> {
  element_id: &'a str,
}

/// `POST /v1/general/delete-advanced-elements` — delete a custom element.
/// Deleted elements show `status: "deleted"` in subsequent queries.
pub async fn delete_element(args: DeleteElementArgs<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let DeleteElementArgs { api_key, element_id } = args;
  if element_id.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("element_id must not be empty".into()).into());
  }
  let body = DeleteElementBody { element_id: &element_id };
  let body_json = serde_json::to_string(&body).map_err(KlingClientError::RequestSerializationError)?;
  legacy_post(api_key, PATH, body_json).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn wire_shape() {
    let body = DeleteElementBody { element_id: "829836802793406551" };
    assert_eq!(
      serde_json::to_value(&body).unwrap(),
      json!({"element_id": "829836802793406551"})
    );
  }

  #[test]
  fn documented_response_parses() {
    let json = r#"{"task_id": "1", "task_status": "succeed"}"#;
    let deleted: LegacyCreatedTask = serde_json::from_str(json).unwrap();
    assert!(deleted.task_status.is_terminal());
  }
}
