use std::error::Error;

/// Every failure mode a send can produce.
#[derive(Debug)]
pub enum EmailError {
  /// Serializing the request body failed (entirely our side).
  RequestSerialization(serde_json::Error),
  /// Transport failure — DNS, TLS, connect/read timeout, etc.
  Transport(reqwest::Error),
  /// Resend answered with a non-2xx status. `body` is the raw response (Resend returns a JSON
  /// `{ statusCode, message, name }` we keep verbatim for logs).
  Api { status: u16, body: String },
}

impl Error for EmailError {}

impl std::fmt::Display for EmailError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::RequestSerialization(e) => write!(f, "email request serialization failed: {e}"),
      Self::Transport(e) => write!(f, "email transport error: {e}"),
      Self::Api { status, body } => write!(f, "email provider returned {status}: {body}"),
    }
  }
}
