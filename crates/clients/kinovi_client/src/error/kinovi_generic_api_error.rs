use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::StatusCode;

/// Grab-bag for server-side responses (and transport failures) that don't map
/// to a [`crate::error::kinovi_specific_api_error::KinoviSpecificApiError`].
#[derive(Debug)]
pub enum KinoviGenericApiError {
  /// Response body didn't deserialize against the expected schema. Includes
  /// the original body so we can diagnose drift.
  SerdeResponseParseErrorWithBody(serde_json::Error, String),

  /// An uncategorized bad HTTP response with status code and body (e.g. 5xx).
  UncategorizedBadResponseWithStatusAndBody {
    status_code: StatusCode,
    body: String,
  },

  /// An uncaught error from the HTTP client (e.g. network/IO/timeout).
  ReqwestError(reqwest::Error),
}

impl Error for KinoviGenericApiError {}

impl Display for KinoviGenericApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::SerdeResponseParseErrorWithBody(err, body) => {
        write!(f, "Failed to parse Kinovi API response body: {:?}. Body: {}", err, body)
      }
      Self::UncategorizedBadResponseWithStatusAndBody { status_code, body } => {
        write!(f, "Uncategorized Kinovi API response: status code {}, body: {}", status_code, body)
      }
      Self::ReqwestError(err) => write!(f, "Reqwest error during Kinovi API call: {}", err),
    }
  }
}
