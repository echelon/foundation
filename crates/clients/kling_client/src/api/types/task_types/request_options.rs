use serde::Serialize;

/// The `options` object accepted by every new-style create endpoint.
///
/// - `callback_url`: Kling POSTs the full task payload (a
///   [`crate::api::types::task_types::kling_task::KlingTask`]) here on every
///   status change. Callbacks are unsigned — treat them as hints and confirm
///   by poll.
/// - `external_task_id`: caller-chosen correlation id, unique per account; it
///   does NOT replace the system task id but can be used to query.
/// - `watermark`: whether outputs carry the Kling watermark (default false;
///   custom watermarks are unsupported).
#[derive(Clone, Debug, Default, Serialize)]
pub struct RequestOptions {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub callback_url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub external_task_id: Option<String>,
  #[serde(rename = "watermark_info", skip_serializing_if = "Option::is_none")]
  pub watermark: Option<WatermarkInfo>,
}

impl RequestOptions {
  /// True when nothing is set — the `options` object is omitted entirely.
  pub(crate) fn is_empty(&self) -> bool {
    self.callback_url.is_none() && self.external_task_id.is_none() && self.watermark.is_none()
  }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct WatermarkInfo {
  pub enabled: bool,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn full_options_wire_shape() {
    let options = RequestOptions {
      callback_url: Some("https://example.com/webhooks/kling".to_string()),
      external_task_id: Some("job_01ABC".to_string()),
      watermark: Some(WatermarkInfo { enabled: false }),
    };
    assert_eq!(
      serde_json::to_value(&options).unwrap(),
      serde_json::json!({
        "callback_url": "https://example.com/webhooks/kling",
        "external_task_id": "job_01ABC",
        "watermark_info": {"enabled": false}
      })
    );
  }

  #[test]
  fn default_is_empty_and_serializes_to_empty_object() {
    let options = RequestOptions::default();
    assert!(options.is_empty());
    assert_eq!(serde_json::to_value(&options).unwrap(), serde_json::json!({}));
  }
}
