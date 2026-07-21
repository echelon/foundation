//! Google SSO via Google Identity Services (accounts.md §3B).
//!
//! The frontend renders the official GIS button (`accounts.google.com/gsi/client`, the
//! current FedCM-based library) and receives a **credential**: an ID token (JWT). We verify
//! it here per Google's checklist (developers.google.com/identity/gsi/web/guides/verify-google-id-token):
//!
//!   - RS256 signature against Google's JWKS (`https://www.googleapis.com/oauth2/v3/certs`),
//!     cached per the endpoint's `Cache-Control: max-age` and refetched on unknown `kid`
//!     (Google rotates these keys);
//!   - `aud` equals OUR client id (a public identifier, not a secret — this flow needs no
//!     client secret at all);
//!   - `iss` is `accounts.google.com` or `https://accounts.google.com`;
//!   - `exp` in the future.
//!
//! Identity is anchored on the token's `sub` (stable, per Google's guidance) — never the
//! email, which users can change. The caller additionally requires `email_verified`.

use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const GOOGLE_ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];
/// Fallback cache lifetime when the JWKS response carries no usable `max-age`.
const DEFAULT_JWKS_TTL: Duration = Duration::from_secs(3600);

/// The verified identity the rest of the auth flow consumes.
#[derive(Debug, Clone)]
pub struct GoogleIdentity {
  /// Google's stable subject id — the identity anchor (`user_sso_connections.provider_user_id`).
  pub sub: String,
  pub email: String,
  pub email_verified: bool,
  pub display_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
  kid: String,
  n: String,
  e: String,
}

#[derive(Debug, Deserialize)]
struct JwkSet {
  keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Claims {
  sub: String,
  email: Option<String>,
  #[serde(default)]
  email_verified: bool,
  name: Option<String>,
}

enum KeySource {
  /// Production: fetch + cache Google's rotating keys.
  Remote { http: reqwest::Client, url: String, cache: RwLock<Option<CachedKeys>> },
  /// Tests: a fixed key set — no network, no secrets.
  Static(Vec<Jwk>),
}

struct CachedKeys {
  fetched_at: Instant,
  ttl: Duration,
  keys: Vec<Jwk>,
}

/// Verifies GIS credentials for one OAuth client id.
pub struct GoogleAuth {
  client_id: String,
  source: KeySource,
}

impl GoogleAuth {
  /// Production verifier against Google's live JWKS endpoint.
  pub fn new(client_id: String) -> Self {
    Self {
      client_id,
      source: KeySource::Remote {
        http: reqwest::Client::new(),
        url: GOOGLE_JWKS_URL.to_string(),
        cache: RwLock::new(None),
      },
    }
  }

  /// Test verifier with an injected JWKS document (`{"keys":[…]}`). Panics on malformed
  /// input — it's a fixture.
  pub fn with_static_jwks(client_id: String, jwks_json: &str) -> Self {
    let set: JwkSet = serde_json::from_str(jwks_json).expect("valid JWKS fixture");
    Self { client_id, source: KeySource::Static(set.keys) }
  }

  /// Verify a GIS credential and extract the identity. Errors are deliberately generic
  /// strings — callers map them to a 401 without leaking verifier internals.
  pub async fn verify(&self, credential: &str) -> Result<GoogleIdentity, String> {
    let header = decode_header(credential).map_err(|e| format!("malformed credential: {e}"))?;
    if header.alg != Algorithm::RS256 {
      return Err(format!("unexpected algorithm: {:?}", header.alg));
    }
    let kid = header.kid.ok_or("credential has no key id")?;
    let key = self.key_for(&kid).await?;
    let decoding_key =
      DecodingKey::from_rsa_components(&key.n, &key.e).map_err(|e| format!("bad JWK components: {e}"))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[&self.client_id]);
    validation.set_issuer(&GOOGLE_ISSUERS);
    // `exp` is validated by default (with small leeway for clock skew).

    let data =
      decode::<Claims>(credential, &decoding_key, &validation).map_err(|e| format!("credential rejected: {e}"))?;
    let email = data.claims.email.ok_or("credential carries no email")?;
    Ok(GoogleIdentity {
      sub: data.claims.sub,
      email,
      email_verified: data.claims.email_verified,
      display_name: data.claims.name,
    })
  }

  async fn key_for(&self, kid: &str) -> Result<Jwk, String> {
    match &self.source {
      KeySource::Static(keys) => {
        keys.iter().find(|k| k.kid == kid).cloned().ok_or_else(|| format!("unknown key id: {kid}"))
      }
      KeySource::Remote { http, url, cache } => {
        // Fresh cache hit?
        if let Some(cached) = cache.read().await.as_ref() {
          if cached.fetched_at.elapsed() < cached.ttl {
            if let Some(k) = cached.keys.iter().find(|k| k.kid == kid) {
              return Ok(k.clone());
            }
            // Known-fresh set without this kid usually means rotation — fall through to refetch.
          }
        }
        let resp = http.get(url).send().await.map_err(|e| format!("JWKS fetch failed: {e}"))?;
        let ttl = parse_max_age(resp.headers()).unwrap_or(DEFAULT_JWKS_TTL);
        let set: JwkSet = resp.json().await.map_err(|e| format!("JWKS parse failed: {e}"))?;
        let found = set.keys.iter().find(|k| k.kid == kid).cloned();
        *cache.write().await = Some(CachedKeys { fetched_at: Instant::now(), ttl, keys: set.keys });
        found.ok_or_else(|| format!("unknown key id: {kid}"))
      }
    }
  }
}

/// `Cache-Control: …, max-age=NNN, …` → TTL (Google's guidance: honor this for re-fetch).
fn parse_max_age(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
  let value = headers.get(reqwest::header::CACHE_CONTROL)?.to_str().ok()?;
  value
    .split(',')
    .filter_map(|d| d.trim().strip_prefix("max-age="))
    .filter_map(|s| s.parse::<u64>().ok())
    .next()
    .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
  use super::*;
  use jsonwebtoken::{encode, EncodingKey, Header};
  use serde::Serialize;

  // Throwaway keypair generated for tests only — NEVER a real credential.
  const TEST_KEY_PEM: &str = include_str!("../../tests/fixtures/google_test_key.pem");
  const TEST_JWKS: &str = include_str!("../../tests/fixtures/google_test_jwks.json");
  const CLIENT_ID: &str = "test-client-id.apps.googleusercontent.com";

  #[derive(Serialize)]
  struct TestClaims<'a> {
    iss: &'a str,
    aud: &'a str,
    sub: &'a str,
    email: &'a str,
    email_verified: bool,
    name: &'a str,
    exp: i64,
    iat: i64,
  }

  fn sign(claims: &TestClaims<'_>, kid: &str) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_string());
    let key = EncodingKey::from_rsa_pem(TEST_KEY_PEM.as_bytes()).unwrap();
    encode(&header, claims, &key).unwrap()
  }

  fn claims<'a>() -> TestClaims<'a> {
    let now = chrono::Utc::now().timestamp();
    TestClaims {
      iss: "https://accounts.google.com",
      aud: CLIENT_ID,
      sub: "108812345678901234567",
      email: "ada@example.com",
      email_verified: true,
      name: "Ada Rivera",
      exp: now + 300,
      iat: now,
    }
  }

  fn verifier() -> GoogleAuth {
    GoogleAuth::with_static_jwks(CLIENT_ID.to_string(), TEST_JWKS)
  }

  #[tokio::test]
  async fn valid_credential_verifies() {
    let token = sign(&claims(), "foundation-test-key");
    let id = verifier().verify(&token).await.unwrap();
    assert_eq!(id.sub, "108812345678901234567");
    assert_eq!(id.email, "ada@example.com");
    assert!(id.email_verified);
    assert_eq!(id.display_name.as_deref(), Some("Ada Rivera"));
  }

  #[tokio::test]
  async fn bare_issuer_form_is_accepted() {
    let mut c = claims();
    c.iss = "accounts.google.com";
    let id = verifier().verify(&sign(&c, "foundation-test-key")).await.unwrap();
    assert_eq!(id.email, "ada@example.com");
  }

  #[tokio::test]
  async fn wrong_audience_is_rejected() {
    let mut c = claims();
    c.aud = "someone-elses-client-id.apps.googleusercontent.com";
    assert!(verifier().verify(&sign(&c, "foundation-test-key")).await.is_err());
  }

  #[tokio::test]
  async fn wrong_issuer_is_rejected() {
    let mut c = claims();
    c.iss = "https://evil.example.com";
    assert!(verifier().verify(&sign(&c, "foundation-test-key")).await.is_err());
  }

  #[tokio::test]
  async fn expired_credential_is_rejected() {
    let mut c = claims();
    c.exp = chrono::Utc::now().timestamp() - 600;
    assert!(verifier().verify(&sign(&c, "foundation-test-key")).await.is_err());
  }

  #[tokio::test]
  async fn unknown_key_id_is_rejected() {
    let token = sign(&claims(), "some-rotated-away-kid");
    assert!(verifier().verify(&token).await.is_err());
  }

  #[tokio::test]
  async fn garbage_and_alg_confusion_are_rejected() {
    assert!(verifier().verify("not-a-jwt").await.is_err());
    // HS256 token signed with the client id as the hmac key — must not pass RS256 validation.
    let hs = encode(
      &Header::new(Algorithm::HS256),
      &claims(),
      &EncodingKey::from_secret(CLIENT_ID.as_bytes()),
    )
    .unwrap();
    assert!(verifier().verify(&hs).await.is_err());
  }

  #[test]
  fn max_age_parses() {
    let mut h = reqwest::header::HeaderMap::new();
    h.insert(reqwest::header::CACHE_CONTROL, "public, max-age=22062, must-revalidate".parse().unwrap());
    assert_eq!(parse_max_age(&h), Some(Duration::from_secs(22062)));
    h.insert(reqwest::header::CACHE_CONTROL, "no-store".parse().unwrap());
    assert_eq!(parse_max_age(&h), None);
  }
}
