//! Shared plumbing for provider clients (codebase_layout_backend.md §5): the reqwest
//! factory and the **four-bucket error taxonomy** every client maps its failures into
//! (conventions_coding.md §3). No DB, no service imports.
//!
//! Organizational pattern (better than ArtCraft's per-client ad-hoc errors): ONE error
//! type, ONE classifier from an HTTP status, and a `GenerationClient` trait so the
//! dispatcher can treat every provider uniformly regardless of sync/async shape.

use std::time::Duration;
use thiserror::Error;

/// The four buckets. Every provider error maps to exactly one; the dispatcher/billing
/// layer only ever needs these four to decide retry vs. refund vs. surface-to-customer.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Bad or missing credentials, or not authorized (typically 401/403). Not retriable; ops alert.
    #[error("auth error: {0}")]
    Auth(String),

    /// The request was rejected as invalid — bad params, moderation, unsupported input
    /// (4xx that isn't auth). Customer-actionable; not retriable as-is.
    #[error("invalid request ({status}): {message}")]
    InvalidRequest { status: u16, message: String },

    /// Provider-side failure — capacity, rate limit, 5xx, upstream timeout. Retriable.
    #[error("provider error ({status}): {message}")]
    Provider { status: u16, message: String },

    /// Transport — connect/timeout/TLS/decode. Retriable.
    #[error("transport error: {0}")]
    Transport(String),
}

impl ClientError {
    /// Whether a retry with backoff could plausibly succeed.
    pub fn is_retriable(&self) -> bool {
        matches!(self, ClientError::Provider { .. } | ClientError::Transport(_))
    }

    /// Map an HTTP status + response body into the taxonomy.
    pub fn from_status(status: u16, body: impl Into<String>) -> ClientError {
        let message = body.into();
        match status {
            401 | 403 => ClientError::Auth(message),
            429 => ClientError::Provider { status, message },
            400..=499 => ClientError::InvalidRequest { status, message },
            _ => ClientError::Provider { status, message },
        }
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        ClientError::Transport(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ClientError>;

/// Build a reqwest client with sane provider-call defaults.
pub fn http_client(user_agent: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .expect("build reqwest client")
}

/// A normalized generation output the dispatcher stores, regardless of provider shape.
#[derive(Debug, Clone)]
pub enum Asset {
    /// Bytes returned inline (e.g. OpenAI/Gemini image base64 decoded).
    Bytes { mime: String, data: Vec<u8> },
    /// A URL to fetch (e.g. Kinovi/Seedance video URL).
    Url { mime: Option<String>, url: String },
}

/// Where an async job is in its lifecycle (Kinovi-style providers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Succeeded,
    Failed,
}

/// Uniform interface over providers. Sync providers (OpenAI, Gemini images) return
/// assets straight from `generate`; async providers (Kinovi video) return a provider
/// job id from `generate` and resolve via `poll`.
pub trait GenerationClient {
    /// Submit a generation. `Ok(Vec<Asset>)` for sync providers, `Ok(vec![])` + a
    /// provider job id via the concrete type for async ones.
    fn provider_name(&self) -> &'static str;
}

/// Read an HTTP response into the taxonomy: non-2xx becomes a classified `ClientError`.
pub async fn read_json<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
    let status = resp.status();
    let bytes = resp.bytes().await?;
    if status.is_success() {
        serde_json::from_slice(&bytes)
            .map_err(|e| ClientError::Transport(format!("decode: {e}")))
    } else {
        let body = String::from_utf8_lossy(&bytes).to_string();
        Err(ClientError::from_status(status.as_u16(), body))
    }
}
