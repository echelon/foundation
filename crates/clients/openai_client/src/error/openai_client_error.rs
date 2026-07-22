use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors that happen entirely on the client side — before, or independent of, the HTTP
/// request reaching OpenAI's servers.
#[derive(Debug)]
pub enum OpenAiClientError {
  /// No API key is present.
  NoApiKeyPresent,

  /// Building the reqwest client failed.
  ReqwestClientError(reqwest::Error),

  /// The request body could not be serialized to JSON.
  RequestSerializationError(serde_json::Error),

  /// Our own pre-flight validation rejected the request before sending — e.g. an empty
  /// prompt, an oversize input image, or an `n` outside 1..=10. No HTTP call was made.
  InvalidRequest(String),

  /// A local file (image / mask for edits) could not be read.
  FileReadError(String),

  /// An inbound webhook failed signature verification (or verified but didn't parse) —
  /// treat the request as unauthenticated and drop it.
  WebhookVerificationFailed(String),

  /// `await_response` gave up: the background response was still non-terminal after the
  /// configured timeout. The job may still finish server-side — poll again with the id.
  PollTimeout { waited: std::time::Duration },
}

impl Error for OpenAiClientError {}

impl Display for OpenAiClientError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NoApiKeyPresent => write!(f, "No API key present."),
      Self::ReqwestClientError(e) => write!(f, "Reqwest client error (during client creation): {e}"),
      Self::RequestSerializationError(e) => write!(f, "Failed to serialize request body to JSON: {e}"),
      Self::InvalidRequest(msg) => write!(f, "Invalid request (rejected client-side before sending): {msg}"),
      Self::FileReadError(msg) => write!(f, "Failed to read a local input file: {msg}"),
      Self::WebhookVerificationFailed(msg) => write!(f, "Webhook verification failed: {msg}"),
      Self::PollTimeout { waited } => {
        write!(f, "Timed out waiting for the background response (waited {waited:?}); it may still complete — keep the response id and poll again.")
      }
    }
  }
}
