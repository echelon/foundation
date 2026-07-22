use std::error::Error;

use crate::error::gemini_client_error::GeminiClientError;
use crate::error::gemini_generic_api_error::GeminiGenericApiError;
use crate::error::gemini_specific_api_error::GeminiSpecificApiError;

/// Top-level error for the Google Gemini image client. Sum of every failure
/// mode any endpoint can produce.
#[derive(Debug)]
pub enum GeminiError {
  Client(GeminiClientError),
  ApiSpecific(GeminiSpecificApiError),
  ApiGeneric(GeminiGenericApiError),
}

impl Error for GeminiError {}

impl std::fmt::Display for GeminiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "GeminiClientError: {:?}", e),
      Self::ApiSpecific(e) => write!(f, "GeminiSpecificApiError: {:?}", e),
      Self::ApiGeneric(e) => write!(f, "GeminiGenericApiError: {:?}", e),
    }
  }
}

impl From<GeminiClientError> for GeminiError {
  fn from(error: GeminiClientError) -> Self {
    Self::Client(error)
  }
}

impl From<GeminiSpecificApiError> for GeminiError {
  fn from(error: GeminiSpecificApiError) -> Self {
    Self::ApiSpecific(error)
  }
}

impl From<GeminiGenericApiError> for GeminiError {
  fn from(error: GeminiGenericApiError) -> Self {
    Self::ApiGeneric(error)
  }
}
