use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors that happen entirely on the client side — before, or independent of,
/// the HTTP request reaching Kling's servers.
#[derive(Debug)]
pub enum KlingClientError {
  /// The request body could not be serialized to JSON.
  RequestSerializationError(serde_json::Error),

  /// Our own pre-flight validation rejected the request before sending — e.g.
  /// an empty prompt, an out-of-range duration, or a settings combination the
  /// endpoint documents as unsupported. No HTTP call was made.
  InvalidRequest(String),

  /// `await_task` gave up: the task was still non-terminal after the
  /// configured timeout. The task may still finish server-side — poll again
  /// with the task id.
  PollTimeout { waited: std::time::Duration },
}

impl Error for KlingClientError {}

impl Display for KlingClientError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::RequestSerializationError(e) => write!(f, "Failed to serialize request body to JSON: {e}"),
      Self::InvalidRequest(msg) => write!(f, "Invalid request (rejected client-side before sending): {msg}"),
      Self::PollTimeout { waited } => {
        write!(f, "Timed out waiting for the Kling task (waited {waited:?}); it may still complete — keep the task id and poll again.")
      }
    }
  }
}
