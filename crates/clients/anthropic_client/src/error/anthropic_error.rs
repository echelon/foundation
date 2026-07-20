use std::error::Error;

use crate::error::anthropic_client_error::AnthropicClientError;
use crate::error::anthropic_generic_api_error::AnthropicGenericApiError;
use crate::error::anthropic_specific_api_error::AnthropicSpecificApiError;

/// Top-level error for the Anthropic client.
#[derive(Debug)]
pub enum AnthropicError {
  Client(AnthropicClientError),
  ApiSpecific(AnthropicSpecificApiError),
  ApiGeneric(AnthropicGenericApiError),
}

impl Error for AnthropicError {}

impl std::fmt::Display for AnthropicError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "AnthropicClientError: {e}"),
      Self::ApiSpecific(e) => write!(f, "AnthropicSpecificApiError: {e}"),
      Self::ApiGeneric(e) => write!(f, "AnthropicGenericApiError: {e}"),
    }
  }
}

impl From<AnthropicClientError> for AnthropicError {
  fn from(e: AnthropicClientError) -> Self {
    Self::Client(e)
  }
}
impl From<AnthropicSpecificApiError> for AnthropicError {
  fn from(e: AnthropicSpecificApiError) -> Self {
    Self::ApiSpecific(e)
  }
}
impl From<AnthropicGenericApiError> for AnthropicError {
  fn from(e: AnthropicGenericApiError) -> Self {
    Self::ApiGeneric(e)
  }
}

impl AnthropicError {
  pub fn is_retriable(&self) -> bool {
    match self {
      Self::ApiSpecific(e) => e.is_retriable(),
      Self::ApiGeneric(_) => true,
      Self::Client(_) => false,
    }
  }
}
