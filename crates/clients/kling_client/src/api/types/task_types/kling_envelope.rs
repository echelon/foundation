use serde::Deserialize;

use crate::error::kling_error::KlingError;
use crate::error::kling_generic_api_error::KlingGenericApiError;

/// Kling's universal response envelope: `{code, message, request_id, data}`.
///
/// `code` 0 means success; non-zero codes follow the documented service-code
/// table (see [`crate::error::kling_specific_api_error::KlingSpecificApiError`]).
/// HTTP-level errors are classified before the envelope is parsed, so a
/// non-zero code here is the belt-and-suspenders case: a 2xx response whose
/// envelope still reports failure.
#[derive(Clone, Debug, Deserialize)]
pub struct KlingEnvelope<T> {
  pub code: i64,
  #[serde(default)]
  pub message: String,
  #[serde(default)]
  pub request_id: String,
  // No `#[serde(default)]` here: on a generic field it would force a
  // `T: Default` bound, and serde already treats a missing `Option` as None.
  pub data: Option<T>,
}

impl<T> KlingEnvelope<T> {
  /// Unwrap the payload, converting a non-zero code or a missing `data` into
  /// a typed error. `raw_body` is attached for log context.
  pub(crate) fn into_data(self, raw_body: &str) -> Result<T, KlingError> {
    if self.code != 0 {
      return Err(
        KlingGenericApiError::EnvelopeErrorCode { service_code: self.code, message: self.message }.into(),
      );
    }
    self.data.ok_or_else(|| {
      KlingGenericApiError::MissingEnvelopeData { body: raw_body.to_string() }.into()
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn success_envelope_unwraps_data() {
    let envelope: KlingEnvelope<Vec<i64>> =
      serde_json::from_str(r#"{"code":0,"message":"SUCCEED","request_id":"req_1","data":[1,2]}"#).unwrap();
    assert_eq!(envelope.into_data("{}").unwrap(), vec![1, 2]);
  }

  #[test]
  fn nonzero_code_is_an_error_even_on_http_2xx() {
    let envelope: KlingEnvelope<Vec<i64>> =
      serde_json::from_str(r#"{"code":1201,"message":"invalid parameter","request_id":"req_1"}"#).unwrap();
    let err = envelope.into_data("{}").unwrap_err();
    assert!(format!("{err}").contains("1201"), "got: {err}");
  }

  #[test]
  fn code_zero_with_missing_data_is_an_error() {
    let envelope: KlingEnvelope<Vec<i64>> =
      serde_json::from_str(r#"{"code":0,"message":"","request_id":"req_1"}"#).unwrap();
    assert!(envelope.into_data("{}").is_err());
  }
}
