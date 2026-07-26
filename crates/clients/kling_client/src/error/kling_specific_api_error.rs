use std::error::Error;
use std::fmt::{Display, Formatter};

/// Well-known, classifiable error responses from the Kling API.
///
/// Kling errors carry both an HTTP status and a service code in the response
/// envelope (`{"code": 1102, "message": …}`); classification uses both — see
/// [`super::classify_kling_http_error`]. The documented service-code table:
///
/// | HTTP | Codes     | Meaning                                             |
/// | ---- | --------- | --------------------------------------------------- |
/// | 401  | 1000–1004 | Authentication failed (bad/expired Authorization)   |
/// | 429  | 1100      | Abnormal account status                             |
/// | 429  | 1101/1102 | In arrears (postpaid) / resource pack exhausted     |
/// | 403  | 1103      | Unauthorized access to the requested resource/model |
/// | 400  | 1200/1201 | Invalid request parameters                          |
/// | 404  | 1202/1203 | Invalid method / resource (e.g. model) not found    |
/// | 400  | 1300/1301 | Platform policy / content security policy triggered |
/// | 429  | 1302–1304 | Rate limit / concurrency limit / IP whitelist       |
/// | 5xx  | 5000–5002 | Server errors (classified as generic, retriable)    |
///
/// Every variant carries the raw HTTP response body (`raw_http_body`) when one
/// was available, so downstream logging, alerting, and pages have the full
/// upstream context — not just our classification.
#[derive(Debug)]
pub enum KlingSpecificApiError {
  /// 401 Unauthorized (service codes 1000–1004) — API key missing, invalid,
  /// not yet valid, or expired.
  Unauthorized {
    raw_http_body: Option<String>,
  },

  /// 403 Forbidden (service code 1103) — the key is valid but not entitled to
  /// the requested resource or model.
  Forbidden {
    raw_http_body: Option<String>,
  },

  /// 404 Not Found (service codes 1202/1203) — unknown method, model, or task.
  NotFound {
    raw_http_body: Option<String>,
  },

  /// 429 with service codes 1101/1102 — account in arrears (postpaid) or
  /// resource package exhausted/expired (prepaid). Needs a recharge, not a
  /// retry.
  InsufficientBalance {
    raw_http_body: Option<String>,
  },

  /// 429 with service code 1100 — abnormal account status; needs account
  /// attention, not a retry.
  AccountAbnormal {
    raw_http_body: Option<String>,
  },

  /// 429 rate/concurrency rejection (service codes 1302–1304, or a 429 with no
  /// classifiable service code). `is_concurrency` is true for code 1303 —
  /// "parallel task over resource pack limit" — Kling's per-package
  /// concurrency cap, distinct from request-rate throttling.
  RateLimited {
    is_concurrency: bool,
    raw_http_body: Option<String>,
  },

  /// 400 with service codes 1300/1301 — the platform's content/security policy
  /// rejected the input. Not retriable with the same content.
  ContentPolicyViolation {
    raw_http_body: Option<String>,
  },

  /// 400 with service codes 1200/1201 (or an unclassified 400) — invalid
  /// request parameters. `reason` is Kling's `message` field when present.
  BadRequest {
    reason: String,
    raw_http_body: Option<String>,
  },
}

impl KlingSpecificApiError {
  /// Rate/concurrency rejections clear on their own; everything else needs a
  /// changed request, a recharge, or account attention.
  pub fn is_retriable(&self) -> bool {
    matches!(self, Self::RateLimited { .. })
  }
}

impl Error for KlingSpecificApiError {}

impl Display for KlingSpecificApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Unauthorized { raw_http_body } => {
        write!(f, "Kling API: Unauthorized (invalid, expired, or missing API key){}", fmt_raw_body(raw_http_body))
      }
      Self::Forbidden { raw_http_body } => {
        write!(f, "Kling API: Forbidden (key not entitled to this resource/model){}", fmt_raw_body(raw_http_body))
      }
      Self::NotFound { raw_http_body } => {
        write!(f, "Kling API: Not found{}", fmt_raw_body(raw_http_body))
      }
      Self::InsufficientBalance { raw_http_body } => {
        write!(f, "Kling API: Insufficient balance (account in arrears or resource pack exhausted){}", fmt_raw_body(raw_http_body))
      }
      Self::AccountAbnormal { raw_http_body } => {
        write!(f, "Kling API: Abnormal account status{}", fmt_raw_body(raw_http_body))
      }
      Self::RateLimited { is_concurrency, raw_http_body } => {
        let kind = if *is_concurrency { "concurrency limit" } else { "rate limit" };
        write!(f, "Kling API: Rate limited ({kind}){}", fmt_raw_body(raw_http_body))
      }
      Self::ContentPolicyViolation { raw_http_body } => {
        write!(f, "Kling API: Content policy violation{}", fmt_raw_body(raw_http_body))
      }
      Self::BadRequest { reason, raw_http_body } => {
        write!(f, "Kling API: Bad request: {reason}{}", fmt_raw_body(raw_http_body))
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
