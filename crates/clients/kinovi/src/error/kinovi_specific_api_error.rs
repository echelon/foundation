use std::error::Error;
use std::fmt::{Display, Formatter};

/// Well-known, classifiable error responses from the Kinovi API.
///
/// Distinguishes things like "your key is bad" from generic 5xx so callers can
/// react appropriately (retry vs. surface to user vs. alert).
///
/// Every variant carries the raw HTTP response body (`raw_http_body`) when one
/// was available, so downstream logging, alerting, and pages have the full
/// upstream context — not just our classification. It's `None` for errors we
/// raise client-side before making the HTTP call.
#[derive(Debug)]
pub enum KinoviSpecificApiError {
  /// 401 Unauthorized — API key is missing, invalid, or revoked.
  Unauthorized {
    raw_http_body: Option<String>,
  },

  /// 402 Payment Required — insufficient credits to fulfill the request.
  InsufficientCredits {
    raw_http_body: Option<String>,
  },

  /// 403 Forbidden — the API key is valid but not permitted to act on this
  /// resource (e.g. polling a task that belongs to a different account).
  Forbidden {
    raw_http_body: Option<String>,
  },

  /// 404 Not Found — typically an unknown `taskId` on the poll endpoint.
  NotFound {
    raw_http_body: Option<String>,
  },

  /// 429 Too Many Requests — concurrency cap or rate limit exceeded.
  RateLimited {
    raw_http_body: Option<String>,
  },

  /// 400 Bad Request — the request body shape was invalid (e.g. unsupported
  /// model, out-of-range duration, bad resolution/aspect ratio).
  BadRequest {
    raw_http_body: Option<String>,
  },
}

impl Error for KinoviSpecificApiError {}

impl Display for KinoviSpecificApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Unauthorized { raw_http_body } => {
        write!(f, "Kinovi API: Unauthorized (invalid or missing API key){}", fmt_raw_body(raw_http_body))
      }
      Self::InsufficientCredits { raw_http_body } => {
        write!(f, "Kinovi API: Insufficient credits{}", fmt_raw_body(raw_http_body))
      }
      Self::Forbidden { raw_http_body } => {
        write!(f, "Kinovi API: Forbidden{}", fmt_raw_body(raw_http_body))
      }
      Self::NotFound { raw_http_body } => {
        write!(f, "Kinovi API: Not found{}", fmt_raw_body(raw_http_body))
      }
      Self::RateLimited { raw_http_body } => {
        write!(f, "Kinovi API: Rate limited{}", fmt_raw_body(raw_http_body))
      }
      Self::BadRequest { raw_http_body } => {
        write!(f, "Kinovi API: Bad request{}", fmt_raw_body(raw_http_body))
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
