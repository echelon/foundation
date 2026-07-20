use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::StatusCode;

/// Grab-bag for server-side responses that don't map to a [`super::openai_specific_api_error::OpenAiSpecificApiError`].
#[derive(Debug)]
pub enum OpenAiGenericApiError {
  /// Response body didn't deserialize against the expected schema. Includes the raw body.
  SerdeResponseParseErrorWithBody(serde_json::Error, String),

  /// An uncategorized bad HTTP response (typically 5xx).
  UncategorizedBadResponseWithStatusAndBody { status_code: StatusCode, body: String },

  /// A network/IO error from the HTTP client.
  ReqwestError(reqwest::Error),
}

impl Error for OpenAiGenericApiError {}

impl Display for OpenAiGenericApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::SerdeResponseParseErrorWithBody(e, body) => {
        write!(f, "Failed to parse OpenAI response body: {e:?}. Body: {body}")
      }
      Self::UncategorizedBadResponseWithStatusAndBody { status_code, body } => {
        write!(f, "Uncategorized OpenAI response: status {status_code}, body: {body}")
      }
      Self::ReqwestError(e) => write!(f, "Reqwest error during OpenAI call: {e}"),
    }
  }
}
