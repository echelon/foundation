use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::StatusCode;

/// Grab-bag for server-side responses that don't map to a
/// `GoogleSpecificApiError`.
#[derive(Debug)]
pub enum GoogleGenericApiError {
  /// Response body didn't deserialize against the expected schema. Includes
  /// the original body so we can diagnose drift.
  SerdeResponseParseErrorWithBody(serde_json::Error, String),

  /// A candidate came back but its inline image data wasn't valid base64.
  Base64DecodeError { detail: String },

  /// An uncategorized bad HTTP response with status code and body.
  UncategorizedBadResponseWithStatusAndBody {
    status_code: StatusCode,
    body: String,
  },

  /// An uncaught error from the HTTP client (e.g. network/IO).
  ReqwestError(reqwest::Error),
}

impl Error for GoogleGenericApiError {}

impl Display for GoogleGenericApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::SerdeResponseParseErrorWithBody(err, body) => {
        write!(f, "Failed to parse Gemini API response body: {:?}. Body: {}", err, body)
      }
      Self::Base64DecodeError { detail } => {
        write!(f, "Failed to base64-decode Gemini inline image data: {}", detail)
      }
      Self::UncategorizedBadResponseWithStatusAndBody { status_code, body } => {
        write!(f, "Uncategorized Gemini API response: status code {}, body: {}", status_code, body)
      }
      Self::ReqwestError(err) => write!(f, "Reqwest error during Gemini API call: {}", err),
    }
  }
}
