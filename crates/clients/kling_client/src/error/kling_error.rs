use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_generic_api_error::KlingGenericApiError;
use crate::error::kling_specific_api_error::KlingSpecificApiError;

/// Top-level error for every Kling binding: exactly one of the three buckets.
#[derive(Debug)]
pub enum KlingError {
  /// Failed before (or independent of) the HTTP exchange.
  Client(KlingClientError),

  /// A well-known, classified Kling API error (auth, balance, policy, …).
  ApiSpecific(KlingSpecificApiError),

  /// An unclassified server-side or transport failure.
  ApiGeneric(KlingGenericApiError),
}

impl Error for KlingError {}

impl Display for KlingError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "{e}"),
      Self::ApiSpecific(e) => write!(f, "{e}"),
      Self::ApiGeneric(e) => write!(f, "{e}"),
    }
  }
}

impl From<KlingClientError> for KlingError {
  fn from(e: KlingClientError) -> Self {
    Self::Client(e)
  }
}

impl From<KlingSpecificApiError> for KlingError {
  fn from(e: KlingSpecificApiError) -> Self {
    Self::ApiSpecific(e)
  }
}

impl From<KlingGenericApiError> for KlingError {
  fn from(e: KlingGenericApiError) -> Self {
    Self::ApiGeneric(e)
  }
}

impl KlingError {
  /// Whether retrying the same request later could plausibly succeed.
  ///
  /// Client-side failures are deterministic (never retriable); generic
  /// server/transport failures are assumed transient; specific errors decide
  /// per variant (rate limits and concurrency rejections are retriable, bad
  /// keys and policy violations are not).
  pub fn is_retriable(&self) -> bool {
    match self {
      Self::Client(_) => false,
      Self::ApiSpecific(e) => e.is_retriable(),
      Self::ApiGeneric(_) => true,
    }
  }
}
