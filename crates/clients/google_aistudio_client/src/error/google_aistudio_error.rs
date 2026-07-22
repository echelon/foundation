use std::error::Error;

use crate::error::google_aistudio_client_error::GoogleAiStudioClientError;
use crate::error::google_aistudio_generic_api_error::GoogleAiStudioGenericApiError;
use crate::error::google_aistudio_specific_api_error::GoogleAiStudioSpecificApiError;

/// Top-level error for the Google Gemini image client. Sum of every failure
/// mode any endpoint can produce.
#[derive(Debug)]
pub enum GoogleAiStudioError {
  Client(GoogleAiStudioClientError),
  ApiSpecific(GoogleAiStudioSpecificApiError),
  ApiGeneric(GoogleAiStudioGenericApiError),
}

impl Error for GoogleAiStudioError {}

impl std::fmt::Display for GoogleAiStudioError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "GoogleAiStudioClientError: {:?}", e),
      Self::ApiSpecific(e) => write!(f, "GoogleAiStudioSpecificApiError: {:?}", e),
      Self::ApiGeneric(e) => write!(f, "GoogleAiStudioGenericApiError: {:?}", e),
    }
  }
}

impl From<GoogleAiStudioClientError> for GoogleAiStudioError {
  fn from(error: GoogleAiStudioClientError) -> Self {
    Self::Client(error)
  }
}

impl From<GoogleAiStudioSpecificApiError> for GoogleAiStudioError {
  fn from(error: GoogleAiStudioSpecificApiError) -> Self {
    Self::ApiSpecific(error)
  }
}

impl From<GoogleAiStudioGenericApiError> for GoogleAiStudioError {
  fn from(error: GoogleAiStudioGenericApiError) -> Self {
    Self::ApiGeneric(error)
  }
}
