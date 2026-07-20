use serde::{Deserialize, Serialize};

// ── Wire request ──

#[derive(Serialize, Debug)]
pub(crate) struct ImageGenerationRequestBody {
  pub model: String,
  pub prompt: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub n: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub size: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub quality: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub background: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output_format: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub moderation: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user: Option<String>,
}

// ── Wire response (shared with image_edit) ──

#[derive(Deserialize, Debug)]
pub(crate) struct ImagesResponseBody {
  pub data: Vec<ImagesResponseDatum>,
  pub output_format: Option<String>,
  pub usage: Option<ImagesUsage>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ImagesResponseDatum {
  pub b64_json: Option<String>,
  pub url: Option<String>,
  pub revised_prompt: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ImagesUsage {
  #[serde(default)]
  pub input_tokens: i64,
  #[serde(default)]
  pub output_tokens: i64,
  #[serde(default)]
  pub total_tokens: i64,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn response_deserializes_b64_and_usage() {
    let json = r#"{
      "data": [ { "b64_json": "aGVsbG8=", "revised_prompt": "a cat" } ],
      "output_format": "png",
      "usage": { "input_tokens": 12, "output_tokens": 300, "total_tokens": 312 }
    }"#;
    let parsed: ImagesResponseBody = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.data.len(), 1);
    assert_eq!(parsed.data[0].b64_json.as_deref(), Some("aGVsbG8="));
    assert_eq!(parsed.output_format.as_deref(), Some("png"));
    assert_eq!(parsed.usage.unwrap().total_tokens, 312);
  }
}
