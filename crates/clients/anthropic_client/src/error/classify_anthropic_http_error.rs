use reqwest::StatusCode;

use crate::error::anthropic_error::AnthropicError;
use crate::error::anthropic_generic_api_error::AnthropicGenericApiError;
use crate::error::anthropic_specific_api_error::AnthropicSpecificApiError;

/// Convert a non-2xx HTTP response into an [`AnthropicError`]. Prefers the `error.type`
/// string in the body, falling back to the status code.
pub fn classify_anthropic_http_error(status: StatusCode, body: Option<&str>) -> Result<(), AnthropicError> {
  if status.is_success() {
    return Ok(());
  }
  let raw = || body.map(str::to_string);
  let err_type = body.and_then(error_type);

  // Prefer the typed Anthropic error kind.
  if let Some(t) = err_type.as_deref() {
    match t {
      "authentication_error" => return Err(AnthropicSpecificApiError::Unauthorized { raw_http_body: raw() }.into()),
      "permission_error" => return Err(AnthropicSpecificApiError::Forbidden { raw_http_body: raw() }.into()),
      "not_found_error" => return Err(AnthropicSpecificApiError::NotFound { raw_http_body: raw() }.into()),
      "request_too_large" => return Err(AnthropicSpecificApiError::RequestTooLarge { raw_http_body: raw() }.into()),
      "rate_limit_error" => return Err(AnthropicSpecificApiError::RateLimited { raw_http_body: raw() }.into()),
      "overloaded_error" => return Err(AnthropicSpecificApiError::Overloaded { raw_http_body: raw() }.into()),
      "invalid_request_error" => {
        return Err(AnthropicSpecificApiError::BadRequest {
          reason: error_message(body.unwrap_or("")).unwrap_or_default(),
          raw_http_body: raw(),
        }
        .into())
      }
      _ => {}
    }
  }

  // Fall back to status codes.
  match status {
    StatusCode::UNAUTHORIZED => Err(AnthropicSpecificApiError::Unauthorized { raw_http_body: raw() }.into()),
    StatusCode::FORBIDDEN => Err(AnthropicSpecificApiError::Forbidden { raw_http_body: raw() }.into()),
    StatusCode::NOT_FOUND => Err(AnthropicSpecificApiError::NotFound { raw_http_body: raw() }.into()),
    StatusCode::PAYLOAD_TOO_LARGE => Err(AnthropicSpecificApiError::RequestTooLarge { raw_http_body: raw() }.into()),
    StatusCode::TOO_MANY_REQUESTS => Err(AnthropicSpecificApiError::RateLimited { raw_http_body: raw() }.into()),
    StatusCode::BAD_REQUEST => Err(AnthropicSpecificApiError::BadRequest {
      reason: error_message(body.unwrap_or("")).unwrap_or_default(),
      raw_http_body: raw(),
    }
    .into()),
    s if s.as_u16() == 529 => Err(AnthropicSpecificApiError::Overloaded { raw_http_body: raw() }.into()),
    _ => Err(AnthropicGenericApiError::UncategorizedBadResponseWithStatusAndBody {
      status_code: status,
      body: body.unwrap_or("").to_string(),
    }
    .into()),
  }
}

fn error_type(body: &str) -> Option<String> {
  let v: serde_json::Value = serde_json::from_str(body).ok()?;
  v.get("error")?.get("type")?.as_str().map(str::to_string)
}
fn error_message(body: &str) -> Option<String> {
  let v: serde_json::Value = serde_json::from_str(body).ok()?;
  v.get("error")?.get("message")?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ok_200() {
    assert!(classify_anthropic_http_error(StatusCode::OK, None).is_ok());
  }

  #[test]
  fn auth_error_by_type() {
    let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
    let e = classify_anthropic_http_error(StatusCode::UNAUTHORIZED, Some(body)).unwrap_err();
    assert!(matches!(e, AnthropicError::ApiSpecific(AnthropicSpecificApiError::Unauthorized { .. })));
  }

  #[test]
  fn rate_limit_by_type() {
    let body = r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#;
    let e = classify_anthropic_http_error(StatusCode::TOO_MANY_REQUESTS, Some(body)).unwrap_err();
    assert!(e.is_retriable());
    assert!(matches!(e, AnthropicError::ApiSpecific(AnthropicSpecificApiError::RateLimited { .. })));
  }

  #[test]
  fn overloaded_529_is_retriable() {
    let body = r#"{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#;
    let e = classify_anthropic_http_error(StatusCode::from_u16(529).unwrap(), Some(body)).unwrap_err();
    assert!(e.is_retriable());
    assert!(matches!(e, AnthropicError::ApiSpecific(AnthropicSpecificApiError::Overloaded { .. })));
  }

  #[test]
  fn invalid_request_extracts_message() {
    let body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"max_tokens: required"}}"#;
    let e = classify_anthropic_http_error(StatusCode::BAD_REQUEST, Some(body)).unwrap_err();
    match e {
      AnthropicError::ApiSpecific(AnthropicSpecificApiError::BadRequest { reason, .. }) => {
        assert_eq!(reason, "max_tokens: required");
      }
      other => panic!("expected BadRequest, got {other:?}"),
    }
  }

  #[test]
  fn server_500_generic() {
    let e = classify_anthropic_http_error(StatusCode::INTERNAL_SERVER_ERROR, Some(r#"{"error":{"type":"api_error"}}"#)).unwrap_err();
    assert!(matches!(e, AnthropicError::ApiGeneric(_)));
  }
}
