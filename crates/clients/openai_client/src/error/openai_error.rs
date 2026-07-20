use std::error::Error;

use crate::error::openai_client_error::OpenAiClientError;
use crate::error::openai_generic_api_error::OpenAiGenericApiError;
use crate::error::openai_specific_api_error::OpenAiSpecificApiError;

/// Top-level error for the OpenAI client. Sum of every failure mode any endpoint
/// can produce: client-side (before the wire), well-known API responses, and the
/// generic catch-all.
#[derive(Debug)]
pub enum OpenAiError {
  Client(OpenAiClientError),
  ApiSpecific(OpenAiSpecificApiError),
  ApiGeneric(OpenAiGenericApiError),
}

impl Error for OpenAiError {}

impl std::fmt::Display for OpenAiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "OpenAiClientError: {e}"),
      Self::ApiSpecific(e) => write!(f, "OpenAiSpecificApiError: {e}"),
      Self::ApiGeneric(e) => write!(f, "OpenAiGenericApiError: {e}"),
    }
  }
}

impl From<OpenAiClientError> for OpenAiError {
  fn from(e: OpenAiClientError) -> Self {
    Self::Client(e)
  }
}
impl From<OpenAiSpecificApiError> for OpenAiError {
  fn from(e: OpenAiSpecificApiError) -> Self {
    Self::ApiSpecific(e)
  }
}
impl From<OpenAiGenericApiError> for OpenAiError {
  fn from(e: OpenAiGenericApiError) -> Self {
    Self::ApiGeneric(e)
  }
}

impl OpenAiError {
  /// Whether a retry with backoff could plausibly succeed (rate limits, 5xx, transport).
  pub fn is_retriable(&self) -> bool {
    match self {
      Self::ApiSpecific(e) => e.is_retriable(),
      Self::ApiGeneric(_) => true,
      Self::Client(_) => false,
    }
  }
}
