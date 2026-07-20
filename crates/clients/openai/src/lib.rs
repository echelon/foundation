//! OpenAI images client — the **GPT Image** family (`gpt-image-1`, `gpt-image-1.5`,
//! `gpt-image-2`).
//!
//! `POST /v1/images/generations`, `Authorization: Bearer`. Synchronous. GPT Image models
//! always return `data[].b64_json` (never a URL), plus a token `usage` block.

use base64::{engine::general_purpose::STANDARD, Engine};
use client_core::{http_client, read_json, Asset, ClientError, Result};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.openai.com/v1";

pub struct OpenAiClient {
    http: reqwest::Client,
    api_key: String,
}

pub struct ImageRequest<'a> {
    /// Provider model id, e.g. `gpt-image-1`.
    pub model_id: &'a str,
    pub prompt: &'a str,
    /// e.g. "1024x1024", "1536x1024", "auto".
    pub size: Option<&'a str>,
    /// "low" | "medium" | "high" | "auto".
    pub quality: Option<&'a str>,
    pub n: u8,
}

/// Token usage returned alongside the images (GPT Image models).
#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

pub struct ImageResult {
    pub assets: Vec<Asset>,
    pub usage: Usage,
    pub output_format: String,
}

impl OpenAiClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: http_client("foundation/openai-client"),
            api_key: api_key.into(),
        }
    }

    pub async fn generate_image(&self, req: ImageRequest<'_>) -> Result<ImageResult> {
        let body = GenerationsRequest {
            model: req.model_id.to_string(),
            prompt: req.prompt.to_string(),
            size: req.size.map(str::to_string),
            quality: req.quality.map(str::to_string),
            n: req.n.max(1),
        };

        let resp = self
            .http
            .post(format!("{BASE_URL}/images/generations"))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let parsed: ImagesResponse = read_json(resp).await?;
        let mime = mime_for(parsed.output_format.as_deref().unwrap_or("png"));
        let mut assets = Vec::new();
        for item in parsed.data {
            if let Some(b64) = item.b64_json {
                let data = STANDARD
                    .decode(b64.as_bytes())
                    .map_err(|e| ClientError::Transport(format!("base64: {e}")))?;
                assets.push(Asset::Bytes {
                    mime: mime.clone(),
                    data,
                });
            } else if let Some(url) = item.url {
                assets.push(Asset::Url {
                    mime: Some(mime.clone()),
                    url,
                });
            }
        }
        let usage = parsed
            .usage
            .map(|u| Usage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
            })
            .unwrap_or_default();

        Ok(ImageResult {
            assets,
            usage,
            output_format: parsed.output_format.unwrap_or_else(|| "png".into()),
        })
    }
}

fn mime_for(fmt: &str) -> String {
    match fmt {
        "jpeg" | "jpg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
    .to_string()
}

/* ── wire types ── */

#[derive(Serialize)]
struct GenerationsRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<String>,
    n: u8,
}

#[derive(Deserialize)]
struct ImagesResponse {
    data: Vec<ImageDatum>,
    output_format: Option<String>,
    usage: Option<UsageIn>,
}

#[derive(Deserialize)]
struct ImageDatum {
    b64_json: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
struct UsageIn {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
}
