use std::error::Error;
use std::fmt::{Display, Formatter};

/// Well-known, classifiable error responses from the Google Gemini image API.
///
/// Distinguishes things like "your key is bad" from generic 5xx so callers can
/// react appropriately (retry vs. surface to user vs. alert).
///
/// Every variant carries the raw HTTP response body (`raw_http_body`) when one
/// was available, so downstream logging, alerting, and pages have the full
/// upstream context — not just our classification. It's `None` for errors we
/// raise client-side before making the HTTP call (e.g. pre-flight validation).
///
/// Google returns its canonical error envelope:
///
/// ```json
/// { "error": { "code": 400, "message": "...", "status": "INVALID_ARGUMENT" } }
/// ```
#[derive(Debug)]
pub enum GoogleSpecificApiError {
  /// 401 / 403 `PERMISSION_DENIED` — API key is missing, invalid, revoked, or
  /// lacks access to the requested model.
  Unauthorized {
    raw_http_body: Option<String>,
  },

  /// 404 `NOT_FOUND` — typically an unknown model id.
  NotFound {
    raw_http_body: Option<String>,
  },

  /// 429 `RESOURCE_EXHAUSTED` — rate limit / quota exceeded.
  RateLimited {
    raw_http_body: Option<String>,
  },

  /// The prompt was rejected by Google's safety filters. Google surfaces this
  /// as a 400 `INVALID_ARGUMENT` whose message mentions safety/blocked/prohibited.
  PromptModerated {
    /// The reason we parsed out of the response (e.g. the `error.message`).
    reason: String,
    /// The raw HTTP response body.
    raw_http_body: Option<String>,
  },

  /// 400 `INVALID_ARGUMENT` — the request body shape was invalid (e.g. missing
  /// required field, unsupported aspect ratio, malformed inline data).
  BadRequest {
    /// The reason we parsed out of the response (or the raw body when it
    /// wasn't parseable). For client-side pre-flight failures, the message
    /// we generated.
    reason: String,
    /// The raw HTTP response body. `None` for client-side pre-flight failures.
    raw_http_body: Option<String>,
  },
}

impl Error for GoogleSpecificApiError {}

impl Display for GoogleSpecificApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Unauthorized { raw_http_body } => {
        write!(f, "Gemini API: Unauthorized (invalid or missing API key){}", fmt_raw_body(raw_http_body))
      }
      Self::NotFound { raw_http_body } => {
        write!(f, "Gemini API: Not found (unknown model?){}", fmt_raw_body(raw_http_body))
      }
      Self::RateLimited { raw_http_body } => {
        write!(f, "Gemini API: Rate limited / quota exhausted{}", fmt_raw_body(raw_http_body))
      }
      Self::PromptModerated { reason, raw_http_body } => {
        write!(f, "Gemini API: Prompt moderated: {}{}", reason, fmt_raw_body(raw_http_body))
      }
      Self::BadRequest { reason, raw_http_body } => {
        write!(f, "Gemini API: Bad request: {}{}", reason, fmt_raw_body(raw_http_body))
      }
    }
  }
}

/// Append the raw HTTP body to a `Display` message when one is present, so
/// `{}`-formatted logs/pages carry the upstream context. Empty when absent.
fn fmt_raw_body(raw_http_body: &Option<String>) -> String {
  match raw_http_body {
    Some(body) if !body.is_empty() => format!(" [raw http body: {}]", body),
    _ => String::new(),
  }
}
