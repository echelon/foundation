use serde::{Deserialize, Serialize};

// ── Request (wire shape) ──
//
// {
//   "contents": [ { "parts": [ {"text": "..."},
//                              {"inline_data": {"mime_type": "image/png", "data": "<b64>"}} ] } ],
//   "generationConfig": {
//     "responseModalities": ["IMAGE"],
//     "candidateCount": 1,
//     "imageConfig": { "aspectRatio": "16:9", "imageSize": "2K" }
//   }
// }

#[derive(Serialize, Debug)]
pub(crate) struct GenerateContentRequestBody {
  pub contents: Vec<Content>,
  #[serde(rename = "generationConfig")]
  pub generation_config: GenerationConfig,
}

#[derive(Serialize, Debug)]
pub(crate) struct Content {
  pub parts: Vec<PartOut>,
}

#[derive(Serialize, Debug)]
pub(crate) struct PartOut {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,

  #[serde(rename = "inline_data", skip_serializing_if = "Option::is_none")]
  pub inline_data: Option<InlineDataOut>,
}

impl PartOut {
  pub fn text(t: String) -> Self {
    Self { text: Some(t), inline_data: None }
  }

  pub fn inline(mime_type: String, data: String) -> Self {
    Self { text: None, inline_data: Some(InlineDataOut { mime_type, data }) }
  }
}

#[derive(Serialize, Debug)]
pub(crate) struct InlineDataOut {
  pub mime_type: String,
  pub data: String,
}

#[derive(Serialize, Debug)]
pub(crate) struct GenerationConfig {
  #[serde(rename = "responseModalities")]
  pub response_modalities: Vec<String>,

  #[serde(rename = "candidateCount", skip_serializing_if = "Option::is_none")]
  pub candidate_count: Option<u32>,

  #[serde(rename = "imageConfig", skip_serializing_if = "Option::is_none")]
  pub image_config: Option<ImageConfig>,
}

#[derive(Serialize, Debug)]
pub(crate) struct ImageConfig {
  #[serde(rename = "aspectRatio", skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<String>,

  #[serde(rename = "imageSize", skip_serializing_if = "Option::is_none")]
  pub image_size: Option<String>,
}

// ── Response (wire shape) ──
//
// {
//   "candidates": [ { "content": { "parts": [ {"inlineData": {"mimeType": "image/png", "data": "<b64>"}} ] },
//                     "finishReason": "STOP" } ],
//   "usageMetadata": { ... }
// }

#[derive(Deserialize, Debug)]
pub(crate) struct GenerateContentResponseBody {
  pub candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Candidate {
  pub content: ContentIn,

  #[serde(rename = "finishReason")]
  pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ContentIn {
  #[serde(default)]
  pub parts: Vec<PartIn>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PartIn {
  #[serde(rename = "inlineData")]
  pub inline_data: Option<InlineDataIn>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct InlineDataIn {
  #[serde(rename = "mimeType")]
  pub mime_type: String,
  pub data: String,
}
