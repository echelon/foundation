use std::error::Error;
use std::fmt::{Display, Formatter};

/// Well-known, classifiable error responses from the OpenAI API. Every variant carries the
/// raw HTTP body (when available) so downstream logging/alerting has the full upstream
/// context — not just our classification.
#[derive(Debug)]
pub enum OpenAiSpecificApiError {
  /// 401 — API key missing, invalid, or revoked.
  Unauthorized { raw_http_body: Option<String> },

  /// 403 — key valid but lacks permission (region, unverified org for GPT Image, …).
  Forbidden { raw_http_body: Option<String> },

  /// 404 — unknown model or resource.
  NotFound { raw_http_body: Option<String> },

  /// 429 — rate limit OR insufficient quota (`insufficient_quota`). `is_quota` distinguishes.
  RateLimited { is_quota: bool, raw_http_body: Option<String> },

  /// The prompt/content was rejected by moderation (`content_policy_violation` / `moderation`).
  Moderated { reason: String, raw_http_body: Option<String> },

  /// 400 — malformed body: bad param, unsupported model, invalid size/quality, etc.
  BadRequest { reason: String, raw_http_body: Option<String> },
}

impl OpenAiSpecificApiError {
  pub fn is_retriable(&self) -> bool {
    // Rate limits are retriable; a hard quota exhaustion is not; the rest are not.
    matches!(self, Self::RateLimited { is_quota: false, .. })
  }
}

impl Error for OpenAiSpecificApiError {}

impl Display for OpenAiSpecificApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Unauthorized { raw_http_body } => {
        write!(f, "OpenAI: Unauthorized (invalid or missing API key){}", raw(raw_http_body))
      }
      Self::Forbidden { raw_http_body } => write!(f, "OpenAI: Forbidden{}", raw(raw_http_body)),
      Self::NotFound { raw_http_body } => write!(f, "OpenAI: Not found{}", raw(raw_http_body)),
      Self::RateLimited { is_quota, raw_http_body } => {
        let what = if *is_quota { "Insufficient quota" } else { "Rate limited" };
        write!(f, "OpenAI: {what}{}", raw(raw_http_body))
      }
      Self::Moderated { reason, raw_http_body } => {
        write!(f, "OpenAI: Moderated: {reason}{}", raw(raw_http_body))
      }
      Self::BadRequest { reason, raw_http_body } => {
        write!(f, "OpenAI: Bad request: {reason}{}", raw(raw_http_body))
      }
    }
  }
}

fn raw(body: &Option<String>) -> String {
  match body {
    Some(b) if !b.is_empty() => format!(" [raw http body: {b}]"),
    _ => String::new(),
  }
}
