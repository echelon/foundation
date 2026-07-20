//! Google Gemini image client — the **Nano Banana** family
//! (`gemini-2.5-flash-image`, `gemini-3-pro-image`, `gemini-3.1-flash-image`).
//!
//! `POST /v1beta/models/{model}:generateContent`, auth via `x-goog-api-key`. Synchronous:
//! image bytes come back base64 in `candidates[].content.parts[].inlineData`.

use base64::{engine::general_purpose::STANDARD, Engine};
use client_core::{http_client, read_json, Asset, ClientError, Result};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

pub struct GoogleClient {
    http: reqwest::Client,
    api_key: String,
}

/// One image input for edit / reference generation.
pub struct ImageInput {
    pub mime: String,
    pub data: Vec<u8>,
}

pub struct ImageRequest<'a> {
    /// Provider model id, e.g. `gemini-2.5-flash-image`.
    pub model_id: &'a str,
    pub prompt: &'a str,
    /// e.g. "16:9", "1:1" — optional.
    pub aspect_ratio: Option<&'a str>,
    /// Reference/edit inputs (empty for pure text-to-image).
    pub images: Vec<ImageInput>,
}

impl GoogleClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: http_client("foundation/google-client"),
            api_key: api_key.into(),
        }
    }

    /// Generate image(s). Returns decoded bytes (Nano Banana returns inline base64, sync).
    pub async fn generate_image(&self, req: ImageRequest<'_>) -> Result<Vec<Asset>> {
        let mut parts: Vec<Part> = vec![Part::text(req.prompt)];
        for img in &req.images {
            parts.push(Part::inline(&img.mime, STANDARD.encode(&img.data)));
        }

        let body = GenerateRequest {
            contents: vec![Content { parts }],
            generation_config: GenerationConfig {
                response_modalities: vec!["IMAGE".into()],
                image_config: req.aspect_ratio.map(|ar| ImageConfig {
                    aspect_ratio: Some(ar.to_string()),
                }),
            },
        };

        let url = format!("{BASE_URL}/models/{}:generateContent", req.model_id);
        let resp = self
            .http
            .post(url)
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let parsed: GenerateResponse = read_json(resp).await?;
        let mut assets = Vec::new();
        for cand in parsed.candidates.unwrap_or_default() {
            for part in cand.content.parts {
                if let Some(inline) = part.inline_data {
                    let data = STANDARD
                        .decode(inline.data.as_bytes())
                        .map_err(|e| ClientError::Transport(format!("base64: {e}")))?;
                    assets.push(Asset::Bytes {
                        mime: inline.mime_type,
                        data,
                    });
                }
            }
        }
        Ok(assets)
    }
}

/* ── wire types ── */

#[derive(Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "inline_data", skip_serializing_if = "Option::is_none")]
    inline_data: Option<InlineDataOut>,
}

impl Part {
    fn text(t: &str) -> Part {
        Part {
            text: Some(t.to_string()),
            inline_data: None,
        }
    }
    fn inline(mime: &str, b64: String) -> Part {
        Part {
            text: None,
            inline_data: Some(InlineDataOut {
                mime_type: mime.to_string(),
                data: b64,
            }),
        }
    }
}

#[derive(Serialize)]
struct InlineDataOut {
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseModalities")]
    response_modalities: Vec<String>,
    #[serde(rename = "imageConfig", skip_serializing_if = "Option::is_none")]
    image_config: Option<ImageConfig>,
}

#[derive(Serialize)]
struct ImageConfig {
    #[serde(rename = "aspectRatio", skip_serializing_if = "Option::is_none")]
    aspect_ratio: Option<String>,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ContentIn,
}

#[derive(Deserialize)]
struct ContentIn {
    #[serde(default)]
    parts: Vec<PartIn>,
}

#[derive(Deserialize)]
struct PartIn {
    #[serde(rename = "inlineData")]
    inline_data: Option<InlineDataIn>,
}

#[derive(Deserialize)]
struct InlineDataIn {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}
