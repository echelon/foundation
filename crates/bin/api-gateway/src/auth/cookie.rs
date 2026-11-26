//! Session cookie helpers (auth.md §1): `HttpOnly; Secure; SameSite=Lax; Domain=…`.

use axum::http::HeaderMap;

pub const COOKIE_NAME: &str = "foundation_session";

/// Build the `Set-Cookie` value that installs the session token.
pub fn set_cookie(token: &str, domain: &str, secure: bool, max_age_secs: i64) -> String {
  let secure_attr = if secure { "; Secure" } else { "" };
  let domain_attr = if domain.is_empty() { String::new() } else { format!("; Domain={domain}") };
  format!(
    "{COOKIE_NAME}={token}; Path=/; HttpOnly{secure_attr}; SameSite=Lax{domain_attr}; Max-Age={max_age_secs}"
  )
}

/// Build the `Set-Cookie` value that clears the session cookie (logout).
pub fn clear_cookie(domain: &str, secure: bool) -> String {
  set_cookie("", domain, secure, 0)
}

/// Extract the session token from the request's `Cookie` header, if present.
pub fn read_session_token(headers: &HeaderMap) -> Option<String> {
  let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
  for pair in cookie_header.split(';') {
    let pair = pair.trim();
    if let Some(value) = pair.strip_prefix(&format!("{COOKIE_NAME}=")) {
      return Some(value.to_string());
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::http::header::COOKIE;
  use axum::http::HeaderValue;

  #[test]
  fn set_cookie_has_expected_attrs() {
    let c = set_cookie("tok123", ".example.com", true, 3600);
    assert!(c.contains("foundation_session=tok123"));
    assert!(c.contains("HttpOnly"));
    assert!(c.contains("Secure"));
    assert!(c.contains("SameSite=Lax"));
    assert!(c.contains("Domain=.example.com"));
    assert!(c.contains("Max-Age=3600"));
  }

  #[test]
  fn insecure_omits_secure() {
    let c = set_cookie("t", "localhost", false, 60);
    assert!(!c.contains("Secure"));
  }

  #[test]
  fn empty_domain_sets_and_clears_host_only_cookies() {
    assert_eq!(set_cookie("token", "", true, 60),
      "foundation_session=token; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=60");
    assert_eq!(clear_cookie("", true),
      "foundation_session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0");
  }

  #[test]
  fn read_extracts_token_among_others() {
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, HeaderValue::from_static("other=1; foundation_session=abc.def.ghi; x=2"));
    assert_eq!(read_session_token(&headers).as_deref(), Some("abc.def.ghi"));
  }

  #[test]
  fn read_none_when_absent() {
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, HeaderValue::from_static("other=1"));
    assert!(read_session_token(&headers).is_none());
  }
}
