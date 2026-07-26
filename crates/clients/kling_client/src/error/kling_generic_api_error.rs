use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::StatusCode;

/// Grab-bag for server-side responses that don't map to a [`super::kling_specific_api_error::KlingSpecificApiError`].
#[derive(Debug)]
pub enum KlingGenericApiError {
  /// Response body didn't deserialize against the expected schema. Includes the raw body.
  SerdeResponseParseErrorWithBody(serde_json::Error, String),

  /// An uncategorized bad HTTP response (typically 5xx — Kling service codes
  /// 5000/5001/5002 all mean "try again later").
  UncategorizedBadResponseWithStatusAndBody { status_code: StatusCode, body: String },

  /// A 2xx response whose envelope carried a non-zero, unclassified service
  /// code — the HTTP layer said success but the Kling envelope disagreed.
  EnvelopeErrorCode { service_code: i64, message: String },

  /// A 2xx response with envelope code 0 but no `data` payload — the response
  /// parsed but had nothing where the result belongs.
  MissingEnvelopeData { body: String },

  /// A network/IO error from the HTTP client.
  ReqwestError(reqwest::Error),
}

impl Error for KlingGenericApiError {}

impl Display for KlingGenericApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::SerdeResponseParseErrorWithBody(e, body) => {
        write!(f, "Failed to parse Kling response body: {e:?}. Body: {body}")
      }
      Self::UncategorizedBadResponseWithStatusAndBody { status_code, body } => {
        write!(f, "Uncategorized Kling response: status {status_code}, body: {body}")
      }
      Self::EnvelopeErrorCode { service_code, message } => {
        write!(f, "Kling envelope reported error code {service_code}: {message}")
      }
      Self::MissingEnvelopeData { body } => {
        write!(f, "Kling envelope had code 0 but no data payload. Body: {body}")
      }
      Self::ReqwestError(e) => write!(f, "Reqwest error during Kling call: {e}"),
    }
  }
}
