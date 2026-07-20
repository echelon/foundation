//! The session token (auth.md §1): a signed JWT used as a rollup of session state. It is NOT
//! trusted alone — middleware verifies the signature here, then online-checks the `sessions`
//! row and `session_version`, so the token stays fully revocable.
//!
//! NOTE: the design targets EdDSA with `kid` rotation; this first implementation uses HS256
//! with a config secret. Swapping the algorithm is contained to this file.

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Cookie session lifetime.
pub const SESSION_TTL: Duration = Duration::days(30);

/// Claims carried in the session JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
  /// Session id (`sess_…`) — the row looked up on every request.
  pub sid: String,
  /// User id (`user_…`).
  pub uid: String,
  /// The user's `session_version` at mint time — the O(1) logout vector clock.
  pub sv: i32,
  pub iat: i64,
  pub exp: i64,
}

/// Signing/verifying keys. One secret in this HS256 implementation.
pub struct JwtKeys {
  encoding: EncodingKey,
  decoding: DecodingKey,
}

impl JwtKeys {
  pub fn new(secret: &str) -> Self {
    Self {
      encoding: EncodingKey::from_secret(secret.as_bytes()),
      decoding: DecodingKey::from_secret(secret.as_bytes()),
    }
  }

  /// Mint a token for a freshly-created session.
  pub fn mint(&self, sid: &str, uid: &str, sv: i32) -> Result<String, String> {
    let now = Utc::now();
    let claims = SessionClaims {
      sid: sid.to_string(),
      uid: uid.to_string(),
      sv,
      iat: now.timestamp(),
      exp: (now + SESSION_TTL).timestamp(),
    };
    encode(&Header::new(Algorithm::HS256), &claims, &self.encoding).map_err(|e| format!("jwt encode: {e}"))
  }

  /// Verify a token's signature + expiry. Returns the claims (still to be online-checked).
  pub fn verify(&self, token: &str) -> Option<SessionClaims> {
    decode::<SessionClaims>(token, &self.decoding, &Validation::new(Algorithm::HS256))
      .ok()
      .map(|d| d.claims)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn mint_then_verify_roundtrip() {
    let keys = JwtKeys::new("test-secret");
    let token = keys.mint("sess_1", "user_1", 3).unwrap();
    let claims = keys.verify(&token).expect("should verify");
    assert_eq!(claims.sid, "sess_1");
    assert_eq!(claims.uid, "user_1");
    assert_eq!(claims.sv, 3);
    assert!(claims.exp > claims.iat);
  }

  #[test]
  fn wrong_secret_fails_verification() {
    let token = JwtKeys::new("secret-a").mint("sess_1", "user_1", 0).unwrap();
    assert!(JwtKeys::new("secret-b").verify(&token).is_none());
  }

  #[test]
  fn tampered_token_fails() {
    let keys = JwtKeys::new("s");
    let mut token = keys.mint("sess_1", "user_1", 0).unwrap();
    token.push('x');
    assert!(keys.verify(&token).is_none());
  }
}
