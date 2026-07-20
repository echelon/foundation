use std::error::Error;

use crate::error::kinovi_client_error::KinoviClientError;
use crate::error::kinovi_generic_api_error::KinoviGenericApiError;
use crate::error::kinovi_specific_api_error::KinoviSpecificApiError;

/// Top-level error for the Kinovi client. Sum of every failure mode any
/// endpoint can produce.
#[derive(Debug)]
pub enum KinoviError {
  /// Failures that happen entirely on our side (no HTTP call, or building it).
  Client(KinoviClientError),
  /// Well-known, classifiable HTTP error responses (401/402/403/404/429/400).
  ApiSpecific(KinoviSpecificApiError),
  /// Everything else the server (or transport) can throw at us.
  ApiGeneric(KinoviGenericApiError),
}

impl Error for KinoviError {}

impl std::fmt::Display for KinoviError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "KinoviClientError: {:?}", e),
      Self::ApiSpecific(e) => write!(f, "KinoviSpecificApiError: {:?}", e),
      Self::ApiGeneric(e) => write!(f, "KinoviGenericApiError: {:?}", e),
    }
  }
}

impl From<KinoviClientError> for KinoviError {
  fn from(error: KinoviClientError) -> Self {
    Self::Client(error)
  }
}

impl From<KinoviSpecificApiError> for KinoviError {
  fn from(error: KinoviSpecificApiError) -> Self {
    Self::ApiSpecific(error)
  }
}

impl From<KinoviGenericApiError> for KinoviError {
  fn from(error: KinoviGenericApiError) -> Self {
    Self::ApiGeneric(error)
  }
}
