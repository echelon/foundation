use std::error::Error;

use crate::error::google_client_error::GoogleClientError;
use crate::error::google_generic_api_error::GoogleGenericApiError;
use crate::error::google_specific_api_error::GoogleSpecificApiError;

/// Top-level error for the Google Gemini image client. Sum of every failure
/// mode any endpoint can produce.
#[derive(Debug)]
pub enum GoogleError {
  Client(GoogleClientError),
  ApiSpecific(GoogleSpecificApiError),
  ApiGeneric(GoogleGenericApiError),
}

impl Error for GoogleError {}

impl std::fmt::Display for GoogleError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "GoogleClientError: {:?}", e),
      Self::ApiSpecific(e) => write!(f, "GoogleSpecificApiError: {:?}", e),
      Self::ApiGeneric(e) => write!(f, "GoogleGenericApiError: {:?}", e),
    }
  }
}

impl From<GoogleClientError> for GoogleError {
  fn from(error: GoogleClientError) -> Self {
    Self::Client(error)
  }
}

impl From<GoogleSpecificApiError> for GoogleError {
  fn from(error: GoogleSpecificApiError) -> Self {
    Self::ApiSpecific(error)
  }
}

impl From<GoogleGenericApiError> for GoogleError {
  fn from(error: GoogleGenericApiError) -> Self {
    Self::ApiGeneric(error)
  }
}
