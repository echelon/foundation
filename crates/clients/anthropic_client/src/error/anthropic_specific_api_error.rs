use std::error::Error;
use std::fmt::{Display, Formatter};

/// Well-known Anthropic error responses. Anthropic's envelope is
/// `{ "type": "error", "error": { "type": "<kind>", "message": "..." } }`; the `error.type`
/// string is the authoritative classifier (status codes corroborate).
#[derive(Debug)]
pub enum AnthropicSpecificApiError {
  /// 401 `authentication_error`.
  Unauthorized { raw_http_body: Option<String> },
  /// 403 `permission_error`.
  Forbidden { raw_http_body: Option<String> },
  /// 404 `not_found_error` — unknown model, typically.
  NotFound { raw_http_body: Option<String> },
  /// 413 `request_too_large`.
  RequestTooLarge { raw_http_body: Option<String> },
  /// 429 `rate_limit_error`.
  RateLimited { raw_http_body: Option<String> },
  /// 529 `overloaded_error` — Anthropic temporarily overloaded (retriable).
  Overloaded { raw_http_body: Option<String> },
  /// 400 `invalid_request_error` — malformed body / bad params.
  BadRequest { reason: String, raw_http_body: Option<String> },
}

impl AnthropicSpecificApiError {
  pub fn is_retriable(&self) -> bool {
    matches!(self, Self::RateLimited { .. } | Self::Overloaded { .. })
  }
}

impl Error for AnthropicSpecificApiError {}

impl Display for AnthropicSpecificApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Unauthorized { raw_http_body } => write!(f, "Anthropic: Unauthorized{}", raw(raw_http_body)),
      Self::Forbidden { raw_http_body } => write!(f, "Anthropic: Forbidden{}", raw(raw_http_body)),
      Self::NotFound { raw_http_body } => write!(f, "Anthropic: Not found{}", raw(raw_http_body)),
      Self::RequestTooLarge { raw_http_body } => write!(f, "Anthropic: Request too large{}", raw(raw_http_body)),
      Self::RateLimited { raw_http_body } => write!(f, "Anthropic: Rate limited{}", raw(raw_http_body)),
      Self::Overloaded { raw_http_body } => write!(f, "Anthropic: Overloaded{}", raw(raw_http_body)),
      Self::BadRequest { reason, raw_http_body } => write!(f, "Anthropic: Bad request: {reason}{}", raw(raw_http_body)),
    }
  }
}

fn raw(body: &Option<String>) -> String {
  match body {
    Some(b) if !b.is_empty() => format!(" [raw http body: {b}]"),
    _ => String::new(),
  }
}
