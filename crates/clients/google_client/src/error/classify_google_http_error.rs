use reqwest::StatusCode;

use crate::error::google_error::GoogleError;
use crate::error::google_generic_api_error::GoogleGenericApiError;
use crate::error::google_specific_api_error::GoogleSpecificApiError;

/// Convert a non-2xx HTTP response into a `GoogleError`.
///
/// On 2xx returns `Ok(())`; on any error status returns the most-specific
/// `GoogleError` variant we can classify from the status code and (optional)
/// response body.
///
/// Google's error envelope is:
///
/// ```json
/// { "error": { "code": 400, "message": "...", "status": "INVALID_ARGUMENT" } }
/// ```
///
/// Status mapping (per Google docs):
///   - 400 `INVALID_ARGUMENT`   → `BadRequest` (or `PromptModerated` if the
///                                 message indicates a safety block)
///   - 401 / 403 `PERMISSION_DENIED` → `Unauthorized`
///   - 404 `NOT_FOUND`          → `NotFound`
///   - 429 `RESOURCE_EXHAUSTED` → `RateLimited`
///   - everything else          → generic catch-all
pub fn classify_google_http_error(status_code: StatusCode, maybe_body: Option<&str>) -> Result<(), GoogleError> {
  if status_code.is_success() {
    return Ok(());
  }

  // Owned copy of the raw response body, attached to every error variant so
  // downstream logging/alerting/pages have the full upstream context.
  let raw_body = || maybe_body.map(|b| b.to_string());

  match status_code {
    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
      return Err(GoogleSpecificApiError::Unauthorized { raw_http_body: raw_body() }.into());
    }
    StatusCode::NOT_FOUND => {
      return Err(GoogleSpecificApiError::NotFound { raw_http_body: raw_body() }.into());
    }
    StatusCode::TOO_MANY_REQUESTS => {
      return Err(GoogleSpecificApiError::RateLimited { raw_http_body: raw_body() }.into());
    }
    _ => {}
  }

  // 400 INVALID_ARGUMENT is usually a malformed body or a safety rejection.
  if status_code == StatusCode::BAD_REQUEST {
    if let Some(body) = maybe_body {
      if body_indicates_moderation(body) {
        let message = extract_google_error_message(body).unwrap_or_else(|| body.to_string());
        return Err(GoogleSpecificApiError::PromptModerated { reason: message, raw_http_body: raw_body() }.into());
      }
      let message = extract_google_error_message(body).unwrap_or_else(|| body.to_string());
      return Err(GoogleSpecificApiError::BadRequest { reason: message, raw_http_body: raw_body() }.into());
    }
    return Err(GoogleSpecificApiError::BadRequest { reason: String::new(), raw_http_body: None }.into());
  }

  // Everything else falls through to the generic catch-all.
  Err(GoogleGenericApiError::UncategorizedBadResponseWithStatusAndBody {
    status_code,
    body: maybe_body.unwrap_or("").to_string(),
  }.into())
}

/// Extract `error.message` from Google's canonical error envelope if present.
fn extract_google_error_message(body: &str) -> Option<String> {
  let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
  parsed.get("error")?.get("message")?.as_str().map(|s| s.to_string())
}

/// Heuristic: does this response body indicate a content-moderation block?
/// Google's safety-filter rejections mention words like "safety", "blocked",
/// or "prohibited".
pub(crate) fn body_indicates_moderation(body: &str) -> bool {
  let lower = body.to_lowercase();
  lower.contains("safety")
    || lower.contains("blocked")
    || lower.contains("prohibited")
    || lower.contains("content policy")
    || lower.contains("content_policy")
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── Success ──

  #[test]
  fn ok_200_returns_ok() {
    assert!(classify_google_http_error(StatusCode::OK, None).is_ok());
  }

  #[test]
  fn created_201_returns_ok() {
    assert!(classify_google_http_error(StatusCode::CREATED, Some("{}")).is_ok());
  }

  // ── Direct status mappings ──

  #[test]
  fn unauthorized_401() {
    let err = classify_google_http_error(StatusCode::UNAUTHORIZED, None).unwrap_err();
    assert!(matches!(err, GoogleError::ApiSpecific(GoogleSpecificApiError::Unauthorized { .. })));
  }

  #[test]
  fn forbidden_403_is_unauthorized() {
    // Google returns 403 PERMISSION_DENIED for a valid-but-unauthorized key.
    let err = classify_google_http_error(StatusCode::FORBIDDEN, Some(r#"{"error":{"status":"PERMISSION_DENIED"}}"#)).unwrap_err();
    assert!(matches!(err, GoogleError::ApiSpecific(GoogleSpecificApiError::Unauthorized { .. })));
  }

  #[test]
  fn not_found_404() {
    let err = classify_google_http_error(StatusCode::NOT_FOUND, None).unwrap_err();
    assert!(matches!(err, GoogleError::ApiSpecific(GoogleSpecificApiError::NotFound { .. })));
  }

  #[test]
  fn rate_limited_429() {
    let err = classify_google_http_error(StatusCode::TOO_MANY_REQUESTS, None).unwrap_err();
    assert!(matches!(err, GoogleError::ApiSpecific(GoogleSpecificApiError::RateLimited { .. })));
  }

  // ── 400 Bad Request ──

  #[test]
  fn bad_request_400_extracts_message() {
    let body = r#"{"error":{"code":400,"message":"Invalid value for aspectRatio","status":"INVALID_ARGUMENT"}}"#;
    let err = classify_google_http_error(StatusCode::BAD_REQUEST, Some(body)).unwrap_err();
    match err {
      GoogleError::ApiSpecific(GoogleSpecificApiError::BadRequest { reason, .. }) => {
        assert_eq!(reason, "Invalid value for aspectRatio");
      }
      other => panic!("expected BadRequest, got: {:?}", other),
    }
  }

  #[test]
  fn bad_request_400_unparseable_body_falls_back_to_raw() {
    let body = "definitely not json";
    let err = classify_google_http_error(StatusCode::BAD_REQUEST, Some(body)).unwrap_err();
    match err {
      GoogleError::ApiSpecific(GoogleSpecificApiError::BadRequest { reason, .. }) => {
        assert_eq!(reason, "definitely not json");
      }
      other => panic!("expected BadRequest, got: {:?}", other),
    }
  }

  #[test]
  fn bad_request_400_no_body_is_empty_string() {
    let err = classify_google_http_error(StatusCode::BAD_REQUEST, None).unwrap_err();
    match err {
      GoogleError::ApiSpecific(GoogleSpecificApiError::BadRequest { reason, .. }) => {
        assert!(reason.is_empty());
      }
      other => panic!("expected BadRequest, got: {:?}", other),
    }
  }

  #[test]
  fn moderation_400_is_prompt_moderated() {
    // Google returns safety blocks as 400 INVALID_ARGUMENT.
    let body = r#"{"error":{"code":400,"message":"The prompt was blocked due to safety concerns","status":"INVALID_ARGUMENT"}}"#;
    let err = classify_google_http_error(StatusCode::BAD_REQUEST, Some(body)).unwrap_err();
    assert!(matches!(err, GoogleError::ApiSpecific(GoogleSpecificApiError::PromptModerated { .. })));
  }

  #[test]
  fn prohibited_400_is_prompt_moderated() {
    let body = r#"{"error":{"message":"This request contains prohibited content"}}"#;
    let err = classify_google_http_error(StatusCode::BAD_REQUEST, Some(body)).unwrap_err();
    assert!(matches!(err, GoogleError::ApiSpecific(GoogleSpecificApiError::PromptModerated { .. })));
  }

  // ── Generic fallback ──

  #[test]
  fn server_error_500_is_generic() {
    let err = classify_google_http_error(StatusCode::INTERNAL_SERVER_ERROR, Some("oops")).unwrap_err();
    assert!(matches!(err, GoogleError::ApiGeneric(GoogleGenericApiError::UncategorizedBadResponseWithStatusAndBody { .. })));
  }

  #[test]
  fn bad_gateway_502_is_generic() {
    let err = classify_google_http_error(StatusCode::BAD_GATEWAY, None).unwrap_err();
    assert!(matches!(err, GoogleError::ApiGeneric(GoogleGenericApiError::UncategorizedBadResponseWithStatusAndBody { .. })));
  }
}
