use reqwest::StatusCode;

use crate::error::openai_error::OpenAiError;
use crate::error::openai_generic_api_error::OpenAiGenericApiError;
use crate::error::openai_specific_api_error::OpenAiSpecificApiError;

/// Convert a non-2xx HTTP response into an [`OpenAiError`]. On 2xx returns `Ok(())`.
pub fn classify_openai_http_error(status: StatusCode, body: Option<&str>) -> Result<(), OpenAiError> {
  if status.is_success() {
    return Ok(());
  }
  let raw = || body.map(str::to_string);

  match status {
    StatusCode::UNAUTHORIZED => return Err(OpenAiSpecificApiError::Unauthorized { raw_http_body: raw() }.into()),
    StatusCode::NOT_FOUND => return Err(OpenAiSpecificApiError::NotFound { raw_http_body: raw() }.into()),
    StatusCode::TOO_MANY_REQUESTS => {
      let is_quota = body.map(body_indicates_quota).unwrap_or(false);
      return Err(OpenAiSpecificApiError::RateLimited { is_quota, raw_http_body: raw() }.into());
    }
    _ => {}
  }

  // 403: forbidden, unless the body flags moderation.
  if status == StatusCode::FORBIDDEN {
    if let Some(b) = body {
      if body_indicates_moderation(b) {
        return Err(OpenAiSpecificApiError::Moderated {
          reason: extract_error_message(b).unwrap_or_else(|| b.to_string()),
          raw_http_body: raw(),
        }
        .into());
      }
    }
    return Err(OpenAiSpecificApiError::Forbidden { raw_http_body: raw() }.into());
  }

  // 400: moderation or a plain bad request.
  if status == StatusCode::BAD_REQUEST {
    if let Some(b) = body {
      if body_indicates_moderation(b) {
        return Err(OpenAiSpecificApiError::Moderated {
          reason: extract_error_message(b).unwrap_or_else(|| b.to_string()),
          raw_http_body: raw(),
        }
        .into());
      }
      let reason = extract_error_message(b).unwrap_or_else(|| b.to_string());
      return Err(OpenAiSpecificApiError::BadRequest { reason, raw_http_body: raw() }.into());
    }
    return Err(OpenAiSpecificApiError::BadRequest { reason: String::new(), raw_http_body: None }.into());
  }

  // Insufficient-quota can arrive as 403/400 with a specific code too; handle 402-ish here.
  Err(OpenAiGenericApiError::UncategorizedBadResponseWithStatusAndBody {
    status_code: status,
    body: body.unwrap_or("").to_string(),
  }
  .into())
}

/// OpenAI error envelope: `{ "error": { "message": "...", "type": "...", "code": "..." } }`.
fn extract_error_message(body: &str) -> Option<String> {
  let v: serde_json::Value = serde_json::from_str(body).ok()?;
  v.get("error")?.get("message")?.as_str().map(str::to_string)
}

fn field_lower(body: &str, field: &str) -> Option<String> {
  let v: serde_json::Value = serde_json::from_str(body).ok()?;
  v.get("error")?.get(field)?.as_str().map(|s| s.to_lowercase())
}

pub(crate) fn body_indicates_moderation(body: &str) -> bool {
  let code = field_lower(body, "code").unwrap_or_default();
  let ty = field_lower(body, "type").unwrap_or_default();
  let lower = body.to_lowercase();
  code.contains("content_policy")
    || code.contains("moderation")
    || ty.contains("content_policy")
    || lower.contains("content policy")
    || lower.contains("safety system")
    || lower.contains("moderation")
}

pub(crate) fn body_indicates_quota(body: &str) -> bool {
  let code = field_lower(body, "code").unwrap_or_default();
  let ty = field_lower(body, "type").unwrap_or_default();
  code.contains("insufficient_quota") || ty.contains("insufficient_quota")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ok_200() {
    assert!(classify_openai_http_error(StatusCode::OK, None).is_ok());
  }

  #[test]
  fn unauthorized_401() {
    let e = classify_openai_http_error(StatusCode::UNAUTHORIZED, None).unwrap_err();
    assert!(matches!(e, OpenAiError::ApiSpecific(OpenAiSpecificApiError::Unauthorized { .. })));
  }

  #[test]
  fn not_found_404() {
    let e = classify_openai_http_error(StatusCode::NOT_FOUND, Some("{}")).unwrap_err();
    assert!(matches!(e, OpenAiError::ApiSpecific(OpenAiSpecificApiError::NotFound { .. })));
  }

  #[test]
  fn rate_limited_plain_429() {
    let e = classify_openai_http_error(StatusCode::TOO_MANY_REQUESTS, Some(r#"{"error":{"type":"rate_limit_exceeded"}}"#)).unwrap_err();
    match e {
      OpenAiError::ApiSpecific(OpenAiSpecificApiError::RateLimited { is_quota, .. }) => assert!(!is_quota),
      other => panic!("expected RateLimited, got {other:?}"),
    }
  }

  #[test]
  fn insufficient_quota_429_is_flagged() {
    let body = r#"{"error":{"type":"insufficient_quota","message":"You exceeded your current quota"}}"#;
    let e = classify_openai_http_error(StatusCode::TOO_MANY_REQUESTS, Some(body)).unwrap_err();
    match e {
      OpenAiError::ApiSpecific(OpenAiSpecificApiError::RateLimited { is_quota, .. }) => assert!(is_quota),
      other => panic!("expected quota RateLimited, got {other:?}"),
    }
  }

  #[test]
  fn moderation_400_is_moderated() {
    let body = r#"{"error":{"code":"content_policy_violation","message":"Your request was rejected by our safety system"}}"#;
    let e = classify_openai_http_error(StatusCode::BAD_REQUEST, Some(body)).unwrap_err();
    assert!(matches!(e, OpenAiError::ApiSpecific(OpenAiSpecificApiError::Moderated { .. })));
  }

  #[test]
  fn bad_request_400_extracts_message() {
    let body = r#"{"error":{"code":"invalid_value","message":"Invalid value for 'size'"}}"#;
    let e = classify_openai_http_error(StatusCode::BAD_REQUEST, Some(body)).unwrap_err();
    match e {
      OpenAiError::ApiSpecific(OpenAiSpecificApiError::BadRequest { reason, .. }) => {
        assert_eq!(reason, "Invalid value for 'size'");
      }
      other => panic!("expected BadRequest, got {other:?}"),
    }
  }

  #[test]
  fn forbidden_403_plain() {
    let e = classify_openai_http_error(StatusCode::FORBIDDEN, Some(r#"{"error":{"message":"unverified org"}}"#)).unwrap_err();
    assert!(matches!(e, OpenAiError::ApiSpecific(OpenAiSpecificApiError::Forbidden { .. })));
  }

  #[test]
  fn server_500_is_generic_and_retriable() {
    let e = classify_openai_http_error(StatusCode::INTERNAL_SERVER_ERROR, Some("oops")).unwrap_err();
    assert!(matches!(e, OpenAiError::ApiGeneric(_)));
    assert!(e.is_retriable());
  }
}
