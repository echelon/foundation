//! The product-API caller extractor (auth.md §5 "two front doors"): an `Authorization`d
//! API key identifies an ACCOUNT (`source = api_key`); a session cookie identifies a USER
//! (`source = dashboard`). Handlers that serve both doors take a [`Caller`].
//!
//! Accepted key forms (api_overview.md §auth — canonical first):
//!   - `Authorization: Bearer foundation_api_…`
//!   - `X-API-Key: foundation_api_…`
//!   - `Authorization: Key foundation_api_…`      (Fal-style)
//!   - `Authorization: foundation_api_…`          (bare)
//!   - `Authorization: Basic base64(key:)`        (Stripe-style, key as username)
//!
//! Query-param keys are NOT read anywhere — they leak into logs and referrers.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;
use base64::Engine;

use db_queries::ApiKey;

use crate::auth::service::sha256_hex;
use crate::error::ApiError;
use crate::state::AppState;

use super::current_user::AuthContext;

const KEY_PREFIX: &str = "foundation_api_";

/// The resolved caller behind one of the two front doors.
#[derive(Debug, Clone)]
pub enum Caller {
  /// Dashboard session (cookie) — a user, who may belong to several accounts.
  Session(AuthContext),
  /// API key — scoped to exactly one account.
  ApiKey(ApiKey),
}

/// Pull a `foundation_api_…` secret out of the headers, if any form of one was sent.
/// Returning `Some` means the caller CHOSE the API-key door — a bad key is then a hard 401,
/// never a silent fall-through to cookie auth.
fn presented_api_key(headers: &HeaderMap) -> Option<String> {
  if let Some(v) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
    let v = v.trim();
    if v.starts_with(KEY_PREFIX) {
      return Some(v.to_string());
    }
  }
  let auth = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?.trim();
  for scheme in ["Bearer ", "Key "] {
    if let Some(rest) = auth.strip_prefix(scheme) {
      let rest = rest.trim();
      if rest.starts_with(KEY_PREFIX) {
        return Some(rest.to_string());
      }
    }
  }
  if auth.starts_with(KEY_PREFIX) {
    return Some(auth.to_string());
  }
  if let Some(b64) = auth.strip_prefix("Basic ") {
    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64.trim()) {
      if let Ok(pair) = String::from_utf8(decoded) {
        let user = pair.split(':').next().unwrap_or("");
        if user.starts_with(KEY_PREFIX) {
          return Some(user.to_string());
        }
      }
    }
  }
  None
}

#[async_trait]
impl FromRequestParts<AppState> for Caller {
  type Rejection = ApiError;

  async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
    if let Some(secret) = presented_api_key(&parts.headers) {
      let key = db_queries::api_keys::find_live_api_key_by_hash(&state.pool, &sha256_hex(&secret))
        .await?
        .ok_or_else(|| ApiError::Unauthorized("invalid API key".into()))?;
      // Coarse freshness stamp, off the request path: writes at most once per 10 min per
      // key (the query itself is conditional); the background job keeps it honest.
      let stale = key.last_used_at.is_none_or(|t| (chrono::Utc::now() - t).num_minutes() >= 10);
      if stale {
        let pool = state.pool.clone();
        let key_id = key.id.clone();
        tokio::spawn(async move {
          if let Err(e) = db_queries::api_keys::touch_api_key_last_used(&pool, &key_id).await {
            tracing::warn!("api key last_used touch failed for {key_id}: {e}");
          }
        });
      }
      return Ok(Caller::ApiKey(key));
    }
    Ok(Caller::Session(AuthContext::from_request_parts(parts, state).await?))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::http::HeaderValue;

  fn headers(name: &str, value: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
      axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
      HeaderValue::from_str(value).unwrap(),
    );
    h
  }

  #[test]
  fn accepts_every_documented_form() {
    let key = "foundation_api_abc123";
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("{key}:"));
    let cases = [
      ("authorization", format!("Bearer {key}")),
      ("authorization", format!("Key {key}")),
      ("authorization", key.to_string()),
      ("authorization", format!("Basic {basic}")),
      ("x-api-key", key.to_string()),
    ];
    for (name, value) in cases {
      assert_eq!(presented_api_key(&headers(name, &value)).as_deref(), Some(key), "form: {name}: {value}");
    }
  }

  #[test]
  fn ignores_non_key_credentials() {
    assert_eq!(presented_api_key(&headers("authorization", "Bearer eyJhbGciOi.jwt.token")), None);
    assert_eq!(presented_api_key(&headers("authorization", "Basic dXNlcjpwYXNz")), None);
    assert_eq!(presented_api_key(&HeaderMap::new()), None);
  }
}
