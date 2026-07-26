use serde::Serialize;

use crate::api::requests::legacy_common::legacy_post;
use crate::api::types::legacy_types::legacy_created_task::LegacyCreatedTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/general/delete-voices";

#[derive(Clone, Debug)]
pub struct DeleteVoiceArgs<'a> {
  pub api_key: &'a KlingApiKey,
  /// Custom voices only — presets can't be deleted.
  pub voice_id: String,
}

#[derive(Serialize)]
pub(crate) struct DeleteVoiceBody<'a> {
  voice_id: &'a str,
}

/// `POST /v1/general/delete-voices` — delete a custom voice. Deleted voices
/// show `status: "deleted"` in subsequent queries.
pub async fn delete_voice(args: DeleteVoiceArgs<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let DeleteVoiceArgs { api_key, voice_id } = args;
  if voice_id.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("voice_id must not be empty".into()).into());
  }
  let body = DeleteVoiceBody { voice_id: &voice_id };
  let body_json = serde_json::to_string(&body).map_err(KlingClientError::RequestSerializationError)?;
  legacy_post(api_key, PATH, body_json).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn wire_shape() {
    let body = DeleteVoiceBody { voice_id: "850087542757535834" };
    assert_eq!(serde_json::to_value(&body).unwrap(), json!({"voice_id": "850087542757535834"}));
  }
}
