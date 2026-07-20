use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::StatusCode;

/// Server responses that don't map to a specific Anthropic error.
#[derive(Debug)]
pub enum AnthropicGenericApiError {
  SerdeResponseParseErrorWithBody(serde_json::Error, String),
  UncategorizedBadResponseWithStatusAndBody { status_code: StatusCode, body: String },
  ReqwestError(reqwest::Error),
}

impl Error for AnthropicGenericApiError {}

impl Display for AnthropicGenericApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::SerdeResponseParseErrorWithBody(e, body) => {
        write!(f, "Failed to parse Anthropic response body: {e:?}. Body: {body}")
      }
      Self::UncategorizedBadResponseWithStatusAndBody { status_code, body } => {
        write!(f, "Uncategorized Anthropic response: status {status_code}, body: {body}")
      }
      Self::ReqwestError(e) => write!(f, "Reqwest error during Anthropic call: {e}"),
    }
  }
}
