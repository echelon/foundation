use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors that happen entirely on the client side — before or independent of
/// the HTTP request reaching Google's servers.
#[derive(Debug)]
pub enum GoogleClientError {
  /// No API key is present.
  NoApiKeyPresent,

  /// An error was encountered building the reqwest client.
  ReqwestClientError(reqwest::Error),

  /// The request body could not be serialized to JSON.
  RequestSerializationError(serde_json::Error),

  /// The request failed our own validation before being sent to Google — e.g.
  /// a required input missing or an empty prompt. Detected client-side; no HTTP
  /// call was made.
  InvalidRequest(String),
}

impl Error for GoogleClientError {}

impl Display for GoogleClientError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NoApiKeyPresent => write!(f, "No API key present."),
      Self::ReqwestClientError(err) => write!(f, "Reqwest client error (during client creation): {}", err),
      Self::RequestSerializationError(err) => write!(f, "Failed to serialize request body to JSON: {}", err),
      Self::InvalidRequest(msg) => write!(f, "Invalid request (rejected client-side before sending): {}", msg),
    }
  }
}
