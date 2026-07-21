//! GitHub SSO via the web application flow (accounts.md §3C) — the modern GitHub-App form:
//! the frontend sends the user to `github.com/login/oauth/authorize` (with `state` and a
//! PKCE challenge, both strongly recommended by GitHub), GitHub redirects back with a `code`,
//! and WE exchange it server-side:
//!
//!   1. `POST https://github.com/login/oauth/access_token` with client id + **client secret**
//!      (+ the PKCE `code_verifier`) → a short-lived user access token (`ghu_…`);
//!   2. `GET /user` → the stable NUMERIC id (the identity anchor — logins are mutable);
//!   3. `GET /user/emails` → the primary **verified** email. `/user`'s `email` field carries
//!      no verified flag, so we never trust it; no verified email = a hard stop (we mark SSO
//!      emails verified). Requires the GitHub App's "Email addresses: read-only" permission.
//!
//! The token is used once for identity and dropped — expiry/refresh are irrelevant here.
//! Endpoints are injectable so tests run against a local mock server: no secrets, no network.

use serde::Deserialize;

const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_API_BASE: &str = "https://api.github.com";
/// GitHub requires a User-Agent on API calls.
const USER_AGENT: &str = "foundation-api-gateway";
const API_VERSION: (&str, &str) = ("X-GitHub-Api-Version", "2022-11-28");

/// The verified identity the rest of the auth flow consumes.
#[derive(Debug, Clone)]
pub struct GithubIdentity {
  /// GitHub's stable numeric user id, stringified — the identity anchor
  /// (`user_sso_connections.provider_user_id`). Never the login, which users can change.
  pub sub: String,
  /// The primary VERIFIED email (from `/user/emails`).
  pub email: String,
  pub display_name: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
  access_token: Option<String>,
  error: Option<String>,
  error_description: Option<String>,
}

#[derive(Deserialize)]
struct GithubUser {
  id: u64,
  login: String,
  name: Option<String>,
}

#[derive(Deserialize)]
struct GithubEmail {
  email: String,
  primary: bool,
  verified: bool,
}

/// Exchanges authorization codes for one GitHub App (client id + secret).
pub struct GithubAuth {
  client_id: String,
  client_secret: String,
  http: reqwest::Client,
  token_url: String,
  api_base: String,
}

impl GithubAuth {
  /// Production exchanger against github.com.
  pub fn new(client_id: String, client_secret: String) -> Self {
    Self::with_endpoints(client_id, client_secret, GITHUB_TOKEN_URL.into(), GITHUB_API_BASE.into())
  }

  /// Test constructor pointing at a mock server.
  pub fn with_endpoints(client_id: String, client_secret: String, token_url: String, api_base: String) -> Self {
    Self { client_id, client_secret, http: reqwest::Client::new(), token_url, api_base }
  }

  /// Resolve an authorization code into a verified identity. Errors are generic strings —
  /// callers map them to a 401 without leaking exchange internals.
  pub async fn resolve_code(&self, code: &str, code_verifier: Option<&str>) -> Result<GithubIdentity, String> {
    // 1. Code → user access token (the client secret lives ONLY in this server-side call).
    let mut form: Vec<(&str, &str)> = vec![
      ("client_id", &self.client_id),
      ("client_secret", &self.client_secret),
      ("code", code),
    ];
    if let Some(verifier) = code_verifier {
      form.push(("code_verifier", verifier));
    }
    let token: TokenResponse = self
      .http
      .post(&self.token_url)
      .header(reqwest::header::ACCEPT, "application/json")
      .header(reqwest::header::USER_AGENT, USER_AGENT)
      .form(&form)
      .send()
      .await
      .map_err(|e| format!("token exchange failed: {e}"))?
      .json()
      .await
      .map_err(|e| format!("token response unreadable: {e}"))?;
    let access_token = match token.access_token {
      Some(t) => t,
      // GitHub reports bad codes as 200 + an error body.
      None => {
        return Err(format!(
          "token exchange rejected: {} ({})",
          token.error.unwrap_or_else(|| "unknown".into()),
          token.error_description.unwrap_or_default()
        ));
      }
    };

    // 2. Who is this? The numeric id is the anchor.
    let user: GithubUser = self
      .api_get(&access_token, "/user")
      .await
      .map_err(|e| format!("GET /user failed: {e}"))?;

    // 3. The primary verified email — mandatory (accounts.md §3C: no verified email → block).
    let emails: Vec<GithubEmail> = self
      .api_get(&access_token, "/user/emails")
      .await
      .map_err(|e| format!("GET /user/emails failed (does the GitHub App have the 'Email addresses: read-only' permission?): {e}"))?;
    let email = emails
      .iter()
      .find(|e| e.primary && e.verified)
      .or_else(|| emails.iter().find(|e| e.verified))
      .map(|e| e.email.clone())
      .ok_or("no verified email on this GitHub account")?;

    Ok(GithubIdentity {
      sub: user.id.to_string(),
      email,
      display_name: user.name.filter(|n| !n.trim().is_empty()).or(Some(user.login)),
    })
  }

  async fn api_get<T: serde::de::DeserializeOwned>(&self, token: &str, path: &str) -> Result<T, String> {
    let resp = self
      .http
      .get(format!("{}{path}", self.api_base))
      .bearer_auth(token)
      .header(reqwest::header::ACCEPT, "application/vnd.github+json")
      .header(reqwest::header::USER_AGENT, USER_AGENT)
      .header(API_VERSION.0, API_VERSION.1)
      .send()
      .await
      .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
      return Err(format!("status {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::extract::Form;
  use axum::routing::{get, post};
  use axum::{Json, Router};
  use serde_json::json;

  const CLIENT_ID: &str = "Iv23test";
  const CLIENT_SECRET: &str = "test-secret-not-real";
  const GOOD_CODE: &str = "good-code";

  /// A local stand-in for github.com + api.github.com: validates the exchange form like the
  /// real thing (bad codes → 200 + error body), serves /user and /user/emails.
  async fn mock_github(emails: serde_json::Value) -> String {
    let token = post(|Form(form): Form<Vec<(String, String)>>| async move {
      let get_field = |k: &str| form.iter().find(|(fk, _)| fk == k).map(|(_, v)| v.as_str());
      if get_field("client_id") != Some(CLIENT_ID) || get_field("client_secret") != Some(CLIENT_SECRET) {
        return Json(json!({ "error": "incorrect_client_credentials" }));
      }
      if get_field("code") != Some(GOOD_CODE) {
        return Json(json!({ "error": "bad_verification_code", "error_description": "The code passed is incorrect or expired." }));
      }
      Json(json!({ "access_token": "ghu_testtoken", "token_type": "bearer", "scope": "" }))
    });
    let user = get(|| async { Json(json!({ "id": 583231, "login": "octocat", "name": "Octo Cat", "email": null })) });
    let emails_route = get(move || {
      let emails = emails.clone();
      async move { Json(emails) }
    });
    let app = Router::new()
      .route("/login/oauth/access_token", token)
      .route("/user", user)
      .route("/user/emails", emails_route);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
  }

  fn auth_for(base: &str) -> GithubAuth {
    GithubAuth::with_endpoints(
      CLIENT_ID.into(),
      CLIENT_SECRET.into(),
      format!("{base}/login/oauth/access_token"),
      base.to_string(),
    )
  }

  #[tokio::test]
  async fn resolves_identity_with_primary_verified_email() {
    let base = mock_github(json!([
      { "email": "octo@example.com", "primary": true, "verified": true },
      { "email": "old@example.com", "primary": false, "verified": true },
    ]))
    .await;
    let id = auth_for(&base).resolve_code(GOOD_CODE, Some("verifier-123")).await.unwrap();
    assert_eq!(id.sub, "583231"); // the numeric id, never the login
    assert_eq!(id.email, "octo@example.com");
    assert_eq!(id.display_name.as_deref(), Some("Octo Cat"));
  }

  #[tokio::test]
  async fn falls_back_to_any_verified_email_when_primary_is_not() {
    let base = mock_github(json!([
      { "email": "unverified@example.com", "primary": true, "verified": false },
      { "email": "second@example.com", "primary": false, "verified": true },
    ]))
    .await;
    let id = auth_for(&base).resolve_code(GOOD_CODE, None).await.unwrap();
    assert_eq!(id.email, "second@example.com");
  }

  #[tokio::test]
  async fn no_verified_email_is_a_hard_stop() {
    let base = mock_github(json!([
      { "email": "unverified@example.com", "primary": true, "verified": false },
    ]))
    .await;
    let err = auth_for(&base).resolve_code(GOOD_CODE, None).await.unwrap_err();
    assert!(err.contains("no verified email"), "{err}");
  }

  #[tokio::test]
  async fn bad_code_is_rejected_even_though_github_answers_200() {
    let base = mock_github(json!([])).await;
    let err = auth_for(&base).resolve_code("stolen-or-expired", None).await.unwrap_err();
    assert!(err.contains("bad_verification_code"), "{err}");
  }

  #[tokio::test]
  async fn wrong_client_credentials_are_rejected() {
    let base = mock_github(json!([])).await;
    let auth = GithubAuth::with_endpoints(
      CLIENT_ID.into(),
      "wrong-secret".into(),
      format!("{base}/login/oauth/access_token"),
      base.clone(),
    );
    let err = auth.resolve_code(GOOD_CODE, None).await.unwrap_err();
    assert!(err.contains("incorrect_client_credentials"), "{err}");
  }
}
