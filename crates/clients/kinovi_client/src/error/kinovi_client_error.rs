use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors that happen entirely on the client side — before or independent of
/// the HTTP request reaching Kinovi's servers.
#[derive(Debug)]
pub enum KinoviClientError {
  /// No API key is present.
  NoApiKeyPresent,

  /// An error was encountered building the reqwest client.
  ReqwestClientError(reqwest::Error),

  /// The request body could not be serialized to JSON.
  RequestSerializationError(serde_json::Error),

  /// The request failed our own validation before being sent to Kinovi — e.g.
  /// an out-of-range duration, too many image URLs, or a missing prompt.
  /// Detected client-side; no HTTP call was made.
  InvalidRequest(String),
}

impl Error for KinoviClientError {}

impl Display for KinoviClientError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NoApiKeyPresent => write!(f, "No API key present."),
      Self::ReqwestClientError(err) => write!(f, "Reqwest client error (during client creation): {}", err),
      Self::RequestSerializationError(err) => write!(f, "Failed to serialize request body to JSON: {}", err),
      Self::InvalidRequest(msg) => write!(f, "Invalid request (rejected client-side before sending): {}", msg),
    }
  }
}
