use reqwest::StatusCode;

use crate::error::kinovi_error::KinoviError;
use crate::error::kinovi_generic_api_error::KinoviGenericApiError;
use crate::error::kinovi_specific_api_error::KinoviSpecificApiError;

/// Convert a non-2xx HTTP response into a [`KinoviError`].
///
/// On 2xx returns `Ok(())`; on any error status returns the most-specific
/// `KinoviError` variant we can classify from the status code. The raw response
/// body (when present) is attached to every variant for downstream context.
///
/// Kinovi's documented status codes:
/// - 400 invalid params
/// - 401 bad key
/// - 402 insufficient credits
/// - 403 unauthorized task (valid key, wrong owner)
/// - 404 task not found
/// - 429 concurrency / rate limit
pub fn classify_kinovi_http_error(status_code: StatusCode, maybe_body: Option<&str>) -> Result<(), KinoviError> {
  if status_code.is_success() {
    return Ok(());
  }

  // Owned copy of the raw response body, attached to every error variant so
  // downstream logging/alerting/pages have the full upstream context.
  let raw_body = || maybe_body.map(|b| b.to_string());

  match status_code {
    StatusCode::UNAUTHORIZED      => Err(KinoviSpecificApiError::Unauthorized { raw_http_body: raw_body() }.into()),
    StatusCode::PAYMENT_REQUIRED  => Err(KinoviSpecificApiError::InsufficientCredits { raw_http_body: raw_body() }.into()),
    StatusCode::FORBIDDEN         => Err(KinoviSpecificApiError::Forbidden { raw_http_body: raw_body() }.into()),
    StatusCode::NOT_FOUND         => Err(KinoviSpecificApiError::NotFound { raw_http_body: raw_body() }.into()),
    StatusCode::TOO_MANY_REQUESTS => Err(KinoviSpecificApiError::RateLimited { raw_http_body: raw_body() }.into()),
    StatusCode::BAD_REQUEST       => Err(KinoviSpecificApiError::BadRequest { raw_http_body: raw_body() }.into()),
    // Everything else falls through to the generic catch-all.
    _ => Err(KinoviGenericApiError::UncategorizedBadResponseWithStatusAndBody {
      status_code,
      body: maybe_body.unwrap_or("").to_string(),
    }.into()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── Success ──

  #[test]
  fn ok_200_returns_ok() {
    assert!(classify_kinovi_http_error(StatusCode::OK, None).is_ok());
  }

  #[test]
  fn created_201_returns_ok() {
    assert!(classify_kinovi_http_error(StatusCode::CREATED, Some("{}")).is_ok());
  }

  // ── Direct status mappings ──

  #[test]
  fn bad_request_400() {
    let err = classify_kinovi_http_error(StatusCode::BAD_REQUEST, Some("bad params")).unwrap_err();
    match err {
      KinoviError::ApiSpecific(KinoviSpecificApiError::BadRequest { raw_http_body }) => {
        assert_eq!(raw_http_body.as_deref(), Some("bad params"));
      }
      other => panic!("expected BadRequest, got: {:?}", other),
    }
  }

  #[test]
  fn unauthorized_401() {
    let err = classify_kinovi_http_error(StatusCode::UNAUTHORIZED, None).unwrap_err();
    assert!(matches!(err, KinoviError::ApiSpecific(KinoviSpecificApiError::Unauthorized { .. })));
  }

  #[test]
  fn payment_required_402() {
    let err = classify_kinovi_http_error(StatusCode::PAYMENT_REQUIRED, None).unwrap_err();
    assert!(matches!(err, KinoviError::ApiSpecific(KinoviSpecificApiError::InsufficientCredits { .. })));
  }

  #[test]
  fn forbidden_403() {
    let err = classify_kinovi_http_error(StatusCode::FORBIDDEN, None).unwrap_err();
    assert!(matches!(err, KinoviError::ApiSpecific(KinoviSpecificApiError::Forbidden { .. })));
  }

  #[test]
  fn not_found_404() {
    let err = classify_kinovi_http_error(StatusCode::NOT_FOUND, None).unwrap_err();
    assert!(matches!(err, KinoviError::ApiSpecific(KinoviSpecificApiError::NotFound { .. })));
  }

  #[test]
  fn rate_limited_429() {
    let err = classify_kinovi_http_error(StatusCode::TOO_MANY_REQUESTS, None).unwrap_err();
    assert!(matches!(err, KinoviError::ApiSpecific(KinoviSpecificApiError::RateLimited { .. })));
  }

  // ── Raw body is threaded through ──

  #[test]
  fn raw_body_is_attached() {
    let err = classify_kinovi_http_error(StatusCode::UNAUTHORIZED, Some("nope")).unwrap_err();
    match err {
      KinoviError::ApiSpecific(KinoviSpecificApiError::Unauthorized { raw_http_body }) => {
        assert_eq!(raw_http_body.as_deref(), Some("nope"));
      }
      other => panic!("expected Unauthorized, got: {:?}", other),
    }
  }

  #[test]
  fn absent_body_is_none() {
    let err = classify_kinovi_http_error(StatusCode::NOT_FOUND, None).unwrap_err();
    match err {
      KinoviError::ApiSpecific(KinoviSpecificApiError::NotFound { raw_http_body }) => {
        assert!(raw_http_body.is_none());
      }
      other => panic!("expected NotFound, got: {:?}", other),
    }
  }

  // ── Generic fallback ──

  #[test]
  fn server_error_500_is_generic() {
    let err = classify_kinovi_http_error(StatusCode::INTERNAL_SERVER_ERROR, Some("oops")).unwrap_err();
    match err {
      KinoviError::ApiGeneric(KinoviGenericApiError::UncategorizedBadResponseWithStatusAndBody { status_code, body }) => {
        assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "oops");
      }
      other => panic!("expected generic, got: {:?}", other),
    }
  }

  #[test]
  fn bad_gateway_502_is_generic() {
    let err = classify_kinovi_http_error(StatusCode::BAD_GATEWAY, None).unwrap_err();
    assert!(matches!(
      err,
      KinoviError::ApiGeneric(KinoviGenericApiError::UncategorizedBadResponseWithStatusAndBody { .. })
    ));
  }
}
