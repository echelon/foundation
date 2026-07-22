use std::error::Error;

use crate::error::google_gemini_client_error::GoogleGeminiClientError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;
use crate::error::google_gemini_specific_api_error::GoogleGeminiSpecificApiError;

/// Top-level error for the Google Gemini image client. Sum of every failure
/// mode any endpoint can produce.
#[derive(Debug)]
pub enum GoogleGeminiError {
  Client(GoogleGeminiClientError),
  ApiSpecific(GoogleGeminiSpecificApiError),
  ApiGeneric(GoogleGeminiGenericApiError),
}

impl Error for GoogleGeminiError {}

impl std::fmt::Display for GoogleGeminiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "GoogleGeminiClientError: {:?}", e),
      Self::ApiSpecific(e) => write!(f, "GoogleGeminiSpecificApiError: {:?}", e),
      Self::ApiGeneric(e) => write!(f, "GoogleGeminiGenericApiError: {:?}", e),
    }
  }
}

impl From<GoogleGeminiClientError> for GoogleGeminiError {
  fn from(error: GoogleGeminiClientError) -> Self {
    Self::Client(error)
  }
}

impl From<GoogleGeminiSpecificApiError> for GoogleGeminiError {
  fn from(error: GoogleGeminiSpecificApiError) -> Self {
    Self::ApiSpecific(error)
  }
}

impl From<GoogleGeminiGenericApiError> for GoogleGeminiError {
  fn from(error: GoogleGeminiGenericApiError) -> Self {
    Self::ApiGeneric(error)
  }
}
