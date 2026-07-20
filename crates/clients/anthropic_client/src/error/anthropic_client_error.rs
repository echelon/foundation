use std::error::Error;
use std::fmt::{Display, Formatter};

/// Client-side failures — before or independent of the HTTP call reaching Anthropic.
#[derive(Debug)]
pub enum AnthropicClientError {
  NoApiKeyPresent,
  ReqwestClientError(reqwest::Error),
  RequestSerializationError(serde_json::Error),
  /// Pre-flight validation failure (empty messages, max_tokens = 0, …). No HTTP call made.
  InvalidRequest(String),
}

impl Error for AnthropicClientError {}

impl Display for AnthropicClientError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NoApiKeyPresent => write!(f, "No API key present."),
      Self::ReqwestClientError(e) => write!(f, "Reqwest client error (during client creation): {e}"),
      Self::RequestSerializationError(e) => write!(f, "Failed to serialize request body to JSON: {e}"),
      Self::InvalidRequest(msg) => write!(f, "Invalid request (rejected client-side before sending): {msg}"),
    }
  }
}
