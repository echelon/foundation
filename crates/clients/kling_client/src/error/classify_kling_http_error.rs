use reqwest::StatusCode;

use crate::error::kling_error::KlingError;
use crate::error::kling_generic_api_error::KlingGenericApiError;
use crate::error::kling_specific_api_error::KlingSpecificApiError;

/// Map an HTTP status + response body to a typed error; `Ok(())` for 2xx.
///
/// Kling doubles up its classification: the HTTP status gives the family and
/// the envelope's `code` field disambiguates within it (e.g. 429 covers
/// account problems 1100–1102 AND throttling 1302–1304). The body is sniffed
/// for the service code; when it can't be parsed, the HTTP status alone
/// decides.
pub fn classify_kling_http_error(status: StatusCode, body: Option<&str>) -> Result<(), KlingError> {
  if status.is_success() {
    return Ok(());
  }

  let raw = || body.map(str::to_string);
  let service_code = body.and_then(extract_service_code);

  match status {
    StatusCode::UNAUTHORIZED => {
      return Err(KlingSpecificApiError::Unauthorized { raw_http_body: raw() }.into());
    }
    StatusCode::FORBIDDEN => {
      return Err(KlingSpecificApiError::Forbidden { raw_http_body: raw() }.into());
    }
    StatusCode::NOT_FOUND => {
      return Err(KlingSpecificApiError::NotFound { raw_http_body: raw() }.into());
    }
    StatusCode::TOO_MANY_REQUESTS => {
      return Err(classify_429(service_code, raw()).into());
    }
    StatusCode::BAD_REQUEST => {
      return Err(classify_400(service_code, body, raw()).into());
    }
    _ => {}
  }

  Err(
    KlingGenericApiError::UncategorizedBadResponseWithStatusAndBody {
      status_code: status,
      body: body.unwrap_or_default().to_string(),
    }
    .into(),
  )
}

/// 429 is overloaded: account exceptions (1100–1102) vs throttling (1302–1304).
fn classify_429(service_code: Option<i64>, raw_http_body: Option<String>) -> KlingSpecificApiError {
  match service_code {
    Some(1100) => KlingSpecificApiError::AccountAbnormal { raw_http_body },
    Some(1101) | Some(1102) => KlingSpecificApiError::InsufficientBalance { raw_http_body },
    Some(1303) => KlingSpecificApiError::RateLimited { is_concurrency: true, raw_http_body },
    _ => KlingSpecificApiError::RateLimited { is_concurrency: false, raw_http_body },
  }
}

/// 400 is overloaded: bad parameters (1200/1201) vs policy triggers (1300/1301).
fn classify_400(service_code: Option<i64>, body: Option<&str>, raw_http_body: Option<String>) -> KlingSpecificApiError {
  match service_code {
    Some(1300) | Some(1301) => KlingSpecificApiError::ContentPolicyViolation { raw_http_body },
    _ => {
      let reason = body
        .and_then(extract_error_message)
        .unwrap_or_else(|| "invalid request parameters".to_string());
      KlingSpecificApiError::BadRequest { reason, raw_http_body }
    }
  }
}

/// Pull the `code` field out of Kling's `{code, message, request_id}` envelope.
fn extract_service_code(body: &str) -> Option<i64> {
  let v: serde_json::Value = serde_json::from_str(body).ok()?;
  v.get("code")?.as_i64()
}

/// Pull the `message` field out of Kling's envelope.
fn extract_error_message(body: &str) -> Option<String> {
  let v: serde_json::Value = serde_json::from_str(body).ok()?;
  let message = v.get("message")?.as_str()?;
  if message.is_empty() {
    return None;
  }
  Some(message.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn envelope(code: i64, message: &str) -> String {
    format!(r#"{{"code":{code},"message":"{message}","request_id":"req_1"}}"#)
  }

  #[test]
  fn success_statuses_pass_through() {
    assert!(classify_kling_http_error(StatusCode::OK, None).is_ok());
    assert!(classify_kling_http_error(StatusCode::CREATED, Some("{}")).is_ok());
  }

  #[test]
  fn unauthorized_is_specific() {
    let err = classify_kling_http_error(StatusCode::UNAUTHORIZED, Some(&envelope(1002, "Authorization is invalid")));
    assert!(matches!(
      err,
      Err(KlingError::ApiSpecific(KlingSpecificApiError::Unauthorized { .. }))
    ));
  }

  #[test]
  fn forbidden_is_specific() {
    let err = classify_kling_http_error(StatusCode::FORBIDDEN, Some(&envelope(1103, "unauthorized model")));
    assert!(matches!(
      err,
      Err(KlingError::ApiSpecific(KlingSpecificApiError::Forbidden { .. }))
    ));
  }

  #[test]
  fn resource_pack_exhausted_is_insufficient_balance_not_rate_limit() {
    let err = classify_kling_http_error(StatusCode::TOO_MANY_REQUESTS, Some(&envelope(1102, "resource pack exhausted")));
    assert!(matches!(
      err,
      Err(KlingError::ApiSpecific(KlingSpecificApiError::InsufficientBalance { .. }))
    ));
  }

  #[test]
  fn account_abnormal_1100_classified() {
    let err = classify_kling_http_error(StatusCode::TOO_MANY_REQUESTS, Some(&envelope(1100, "abnormal account status")));
    assert!(matches!(
      err,
      Err(KlingError::ApiSpecific(KlingSpecificApiError::AccountAbnormal { .. }))
    ));
  }

  #[test]
  fn concurrency_1303_flagged_as_concurrency() {
    let err = classify_kling_http_error(
      StatusCode::TOO_MANY_REQUESTS,
      Some(&envelope(1303, "parallel task over resource pack limit")),
    );
    assert!(matches!(
      err,
      Err(KlingError::ApiSpecific(KlingSpecificApiError::RateLimited { is_concurrency: true, .. }))
    ));
  }

  #[test]
  fn plain_429_without_body_is_rate_limited() {
    let err = classify_kling_http_error(StatusCode::TOO_MANY_REQUESTS, None);
    assert!(matches!(
      err,
      Err(KlingError::ApiSpecific(KlingSpecificApiError::RateLimited { is_concurrency: false, .. }))
    ));
  }

  #[test]
  fn content_policy_1301_classified() {
    let err = classify_kling_http_error(StatusCode::BAD_REQUEST, Some(&envelope(1301, "content security policy")));
    assert!(matches!(
      err,
      Err(KlingError::ApiSpecific(KlingSpecificApiError::ContentPolicyViolation { .. }))
    ));
  }

  #[test]
  fn bad_request_carries_the_message() {
    let err = classify_kling_http_error(StatusCode::BAD_REQUEST, Some(&envelope(1201, "invalid duration")));
    match err {
      Err(KlingError::ApiSpecific(KlingSpecificApiError::BadRequest { reason, .. })) => {
        assert_eq!(reason, "invalid duration");
      }
      other => panic!("expected BadRequest, got: {other:?}"),
    }
  }

  #[test]
  fn not_found_is_specific() {
    let err = classify_kling_http_error(StatusCode::NOT_FOUND, Some(&envelope(1203, "model not found")));
    assert!(matches!(
      err,
      Err(KlingError::ApiSpecific(KlingSpecificApiError::NotFound { .. }))
    ));
  }

  #[test]
  fn server_errors_are_generic_and_retriable() {
    let err = classify_kling_http_error(StatusCode::INTERNAL_SERVER_ERROR, Some(&envelope(5000, "internal error")));
    match err {
      Err(e @ KlingError::ApiGeneric(_)) => assert!(e.is_retriable()),
      other => panic!("expected ApiGeneric, got: {other:?}"),
    }
  }

  #[test]
  fn retriability_matrix() {
    let rate_limited: KlingError = KlingSpecificApiError::RateLimited { is_concurrency: true, raw_http_body: None }.into();
    assert!(rate_limited.is_retriable());

    let balance: KlingError = KlingSpecificApiError::InsufficientBalance { raw_http_body: None }.into();
    assert!(!balance.is_retriable());

    let moderated: KlingError = KlingSpecificApiError::ContentPolicyViolation { raw_http_body: None }.into();
    assert!(!moderated.is_retriable());
  }
}
