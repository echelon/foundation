//! End-to-end auth flow tests through the real router against `foundation_test`.
//!
//! Uses unique emails per run so rows never collide; the test database accumulates harmlessly
//! (reset with `just db-reset-test`). `db::test_fixtures` hard-guards the URL is a `*_test` db.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot

use api_gateway::auth::token::JwtKeys;
use api_gateway::AppState;

async fn app_and_pool() -> (Router, db::PgPool) {
  let pool = db::test_fixtures::create_test_pool().await;
  let state = AppState {
    pool: pool.clone(),
    jwt: Arc::new(JwtKeys::new("test-secret")),
    cookie_domain: "localhost".into(),
    cookie_secure: false,
    app_base_url: "http://localhost:3000".into(),
    email_from: "Foundation <noreply@localhost>".into(),
    resend_api_key: None, // email sends log + no-op in tests
    bucket: None,         // storage off — avatar uploads error, avatar links absent
    cdn_base_url: "http://localhost:9000/foundation".into(),
    environment: config::Environment::Test,
    status_message: None,
    google: None,
  };
  (api_gateway::build_router(state), pool)
}

fn unique_email() -> String {
  format!("user-{}@example.com", id_core::mint("t"))
}

fn post(uri: &str, body: Value, cookie: Option<&str>) -> Request<Body> {
  let mut b = Request::builder().method("POST").uri(uri).header("content-type", "application/json");
  if let Some(c) = cookie {
    b = b.header(header::COOKIE, c);
  }
  b.body(Body::from(serde_json::to_vec(&body).unwrap())).unwrap()
}

fn get(uri: &str, cookie: Option<&str>) -> Request<Body> {
  let mut b = Request::builder().method("GET").uri(uri);
  if let Some(c) = cookie {
    b = b.header(header::COOKIE, c);
  }
  b.body(Body::empty()).unwrap()
}

/// Pull the `foundation_session` cookie back out of a response's Set-Cookie header, as a
/// `Cookie:` header value for the next request.
fn session_cookie(resp: &axum::response::Response) -> Option<String> {
  for v in resp.headers().get_all(header::SET_COOKIE) {
    let s = v.to_str().ok()?;
    if let Some(rest) = s.strip_prefix("foundation_session=") {
      let token = rest.split(';').next().unwrap_or("");
      if token.is_empty() {
        return None; // a cleared cookie
      }
      return Some(format!("foundation_session={token}"));
    }
  }
  None
}

async fn body_json(resp: axum::response::Response) -> Value {
  let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
  serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn signup_sets_cookie_and_me_returns_user() {
  let (app, _pool) = app_and_pool().await;
  let email = unique_email();

  let resp = app
    .clone()
    .oneshot(post("/v1/auth/signup", json!({ "email": email, "password": "hunter2hunter2", "display_name": "Ada" }), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let cookie = session_cookie(&resp).expect("signup should set a session cookie");
  let body = body_json(resp).await;
  assert_eq!(body["user"]["email"], email);

  // /me with the cookie → the same user.
  let me = app.clone().oneshot(get("/v1/auth/me", Some(&cookie))).await.unwrap();
  assert_eq!(me.status(), StatusCode::OK);
  assert_eq!(body_json(me).await["user"]["email"], email);

  // /me without a cookie → 401.
  let anon = app.oneshot(get("/v1/auth/me", None)).await.unwrap();
  assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn short_password_is_rejected() {
  let (app, _pool) = app_and_pool().await;
  let resp = app
    .oneshot(post("/v1/auth/signup", json!({ "email": unique_email(), "password": "short" }), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_succeeds_with_correct_password_and_fails_otherwise() {
  let (app, _pool) = app_and_pool().await;
  let email = unique_email();
  let pw = "correct horse staple";

  app.clone().oneshot(post("/v1/auth/signup", json!({ "email": email, "password": pw }), None)).await.unwrap();

  let ok = app.clone().oneshot(post("/v1/auth/login", json!({ "email": email, "password": pw }), None)).await.unwrap();
  assert_eq!(ok.status(), StatusCode::OK);
  assert!(session_cookie(&ok).is_some());

  let bad = app.clone().oneshot(post("/v1/auth/login", json!({ "email": email, "password": "wrong" }), None)).await.unwrap();
  assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);

  // Unknown email → same generic 401 (no enumeration).
  let unknown = app.oneshot(post("/v1/auth/login", json!({ "email": "nobody@example.com", "password": "x" }), None)).await.unwrap();
  assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn duplicate_signup_with_wrong_password_conflicts() {
  let (app, _pool) = app_and_pool().await;
  let email = unique_email();
  app.clone().oneshot(post("/v1/auth/signup", json!({ "email": email, "password": "firstpass1" }), None)).await.unwrap();

  let dup = app.oneshot(post("/v1/auth/signup", json!({ "email": email, "password": "different!" }), None)).await.unwrap();
  assert_eq!(dup.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn logout_revokes_the_session() {
  let (app, _pool) = app_and_pool().await;
  let email = unique_email();
  let signup = app.clone().oneshot(post("/v1/auth/signup", json!({ "email": email, "password": "logmeout12" }), None)).await.unwrap();
  let cookie = session_cookie(&signup).unwrap();

  // logout
  let out = app.clone().oneshot(post("/v1/auth/logout", json!({}), Some(&cookie))).await.unwrap();
  assert_eq!(out.status(), StatusCode::OK);

  // the same cookie is now rejected (session revoked → online check fails)
  let me = app.oneshot(get("/v1/auth/me", Some(&cookie))).await.unwrap();
  assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

/* ── Google SSO (GIS credential flow — tokens signed with a throwaway TEST key) ── */

const GOOGLE_TEST_KEY_PEM: &str = include_str!("fixtures/google_test_key.pem");
const GOOGLE_TEST_JWKS: &str = include_str!("fixtures/google_test_jwks.json");
const GOOGLE_TEST_CLIENT_ID: &str = "test-client-id.apps.googleusercontent.com";

/// An app whose Google verifier trusts the TEST keypair — full flow, no network, no secrets.
async fn app_with_google() -> (Router, db::PgPool) {
  let pool = db::test_fixtures::create_test_pool().await;
  let state = AppState {
    pool: pool.clone(),
    jwt: Arc::new(JwtKeys::new("test-secret")),
    cookie_domain: "localhost".into(),
    cookie_secure: false,
    app_base_url: "http://localhost:3000".into(),
    email_from: "Foundation <noreply@localhost>".into(),
    resend_api_key: None,
    bucket: None,
    cdn_base_url: "http://localhost:9000/foundation".into(),
    environment: config::Environment::Test,
    status_message: None,
    google: Some(Arc::new(api_gateway::auth::google::GoogleAuth::with_static_jwks(
      GOOGLE_TEST_CLIENT_ID.to_string(),
      GOOGLE_TEST_JWKS,
    ))),
  };
  (api_gateway::build_router(state), pool)
}

fn google_credential(sub: &str, email: &str, name: &str, email_verified: bool) -> String {
  use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
  let now = chrono::Utc::now().timestamp();
  let claims = json!({
    "iss": "https://accounts.google.com",
    "aud": GOOGLE_TEST_CLIENT_ID,
    "sub": sub,
    "email": email,
    "email_verified": email_verified,
    "name": name,
    "iat": now,
    "exp": now + 300,
  });
  let mut header = Header::new(Algorithm::RS256);
  header.kid = Some("foundation-test-key".to_string());
  encode(&header, &claims, &EncodingKey::from_rsa_pem(GOOGLE_TEST_KEY_PEM.as_bytes()).unwrap()).unwrap()
}

#[tokio::test]
async fn google_sso_signs_up_then_logs_in_on_repeat() {
  let (app, pool) = app_with_google().await;
  let email = unique_email();
  let sub = format!("10881{}", id_core::mint("g"));

  // First visit: verified credential → user + own org created, session cookie set.
  let credential = google_credential(&sub, &email, "SSO User", true);
  let first = app
    .clone()
    .oneshot(post("/v1/auth/oauth/google/callback", json!({ "credential": credential }), None))
    .await
    .unwrap();
  assert_eq!(first.status(), StatusCode::OK);
  let cookie = session_cookie(&first).unwrap();
  let body = body_json(first).await;
  assert_eq!(body["user"]["email"], email);
  assert_eq!(body["user"]["display_name"], "SSO User");
  let user_id: ids::UserId = body["user"]["id"].as_str().unwrap().parse().unwrap();

  // SSO emails count as verified, the session works, and created_via says sso_signup.
  let me = app.clone().oneshot(get("/v1/auth/me", Some(&cookie))).await.unwrap();
  assert_eq!(body_json(me).await["user"]["email_verified"], true);
  let sessions = db_queries::sessions::list_live_sessions_for_user(&pool, &user_id).await.unwrap();
  assert_eq!(sessions[0].created_via, enums::SessionCreationType::SsoSignup);

  // Same (provider, sub) again — even with a CHANGED email — logs into the same user.
  let second_credential = google_credential(&sub, &unique_email(), "SSO User", true);
  let second = app
    .clone()
    .oneshot(post("/v1/auth/oauth/google/callback", json!({ "credential": second_credential }), None))
    .await
    .unwrap();
  assert_eq!(second.status(), StatusCode::OK);
  assert_eq!(body_json(second).await["user"]["id"], user_id.to_string());
  let sessions = db_queries::sessions::list_live_sessions_for_user(&pool, &user_id).await.unwrap();
  assert_eq!(sessions[0].created_via, enums::SessionCreationType::SsoLogin);
}

#[tokio::test]
async fn google_sso_rejects_bad_credentials_and_collisions() {
  let (app, _pool) = app_with_google().await;

  // Garbage / unverifiable credential → 401.
  let bad = app
    .clone()
    .oneshot(post("/v1/auth/oauth/google/callback", json!({ "credential": "not-a-real-jwt" }), None))
    .await
    .unwrap();
  assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);

  // Google-unverified email → 401 (we mark SSO emails verified, so we require Google's word).
  let email = unique_email();
  let unverified = google_credential(&format!("10882{}", id_core::mint("g")), &email, "X", false);
  let resp = app
    .clone()
    .oneshot(post("/v1/auth/oauth/google/callback", json!({ "credential": unverified }), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

  // Email already registered with a password and NO Google link → 409, no silent merge.
  let taken = unique_email();
  let signup = app
    .clone()
    .oneshot(post("/v1/auth/signup", json!({ "email": taken, "password": "password1" }), None))
    .await
    .unwrap();
  assert_eq!(signup.status(), StatusCode::CREATED);
  let colliding = google_credential(&format!("10883{}", id_core::mint("g")), &taken, "X", true);
  let resp = app
    .clone()
    .oneshot(post("/v1/auth/oauth/google/callback", json!({ "credential": colliding }), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CONFLICT);

  // Unsupported provider → 400.
  let gh = app
    .clone()
    .oneshot(post("/v1/auth/oauth/github/callback", json!({ "credential": "x" }), None))
    .await
    .unwrap();
  assert_eq!(gh.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn google_sso_unconfigured_server_says_so() {
  let (app, _pool) = app_and_pool().await; // google: None
  let resp = app
    .oneshot(post("/v1/auth/oauth/google/callback", json!({ "credential": "anything" }), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn invite_create_resolve_and_accept() {
  let (app, pool) = app_and_pool().await;

  // Owner signs up.
  let owner_email = unique_email();
  let signup = app.clone().oneshot(post("/v1/auth/signup", json!({ "email": owner_email, "password": "ownerpass1" }), None)).await.unwrap();
  let owner_cookie = session_cookie(&signup).unwrap();
  let owner_id: ids::UserId = body_json(signup).await["user"]["id"].as_str().unwrap().parse().unwrap();

  // Find the owner's account (signup created it + an owner membership).
  let account_id = db_queries::members::list_memberships_for_user(&pool, &owner_id).await.unwrap()[0]
    .account_id
    .clone();

  // Owner creates an invitation.
  let invitee_email = unique_email();
  let inv = app
    .clone()
    .oneshot(post(&format!("/v1/accounts/{account_id}/invitations"), json!({ "email": invitee_email, "role": "member" }), Some(&owner_cookie)))
    .await
    .unwrap();
  assert_eq!(inv.status(), StatusCode::OK);
  let inv_body = body_json(inv).await;
  let code = inv_body["code"].as_str().unwrap().to_string();
  let invitation_id = inv_body["invitation_id"].as_str().unwrap().to_string();

  // Resolve via code (public).
  let resolved = app.clone().oneshot(get(&format!("/v1/invitations/{code}"), None)).await.unwrap();
  assert_eq!(resolved.status(), StatusCode::OK);
  assert_eq!(body_json(resolved).await["role"], "member");

  // Invitee signs up, then accepts.
  let joiner_signup = app.clone().oneshot(post("/v1/auth/signup", json!({ "email": invitee_email, "password": "joinerpass1" }), None)).await.unwrap();
  let joiner_cookie = session_cookie(&joiner_signup).unwrap();
  let joiner_id: ids::UserId = body_json(joiner_signup).await["user"]["id"].as_str().unwrap().parse().unwrap();

  let accept = app.clone().oneshot(post(&format!("/v1/invitations/{invitation_id}/accept"), json!({}), Some(&joiner_cookie))).await.unwrap();
  assert_eq!(accept.status(), StatusCode::OK);

  // The joiner is now a member of the owner's account.
  let mem = db_queries::members::find_membership(&pool, &account_id, &joiner_id).await.unwrap();
  assert_eq!(mem.unwrap().role, "member");

  // A non-member cannot invite (403/404).
  let outsider_email = unique_email();
  let outsider_signup = app.clone().oneshot(post("/v1/auth/signup", json!({ "email": outsider_email, "password": "outsider12" }), None)).await.unwrap();
  let outsider_cookie = session_cookie(&outsider_signup).unwrap();
  let forbidden = app
    .oneshot(post(&format!("/v1/accounts/{account_id}/invitations"), json!({ "email": unique_email(), "role": "member" }), Some(&outsider_cookie)))
    .await
    .unwrap();
  assert!(forbidden.status() == StatusCode::NOT_FOUND || forbidden.status() == StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn api_status_reports_reachability_and_operator_message() {
  // Quiet by default: ok, no message.
  let (app, pool) = app_and_pool().await;
  let resp = app.clone().oneshot(get("/v1/api_status", None)).await.unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let body = body_json(resp).await;
  assert_eq!(body["ok"], true);
  assert_eq!(body["message"], Value::Null);

  // With an operator message (API_STATUS_MESSAGE at deploy) it's surfaced verbatim.
  let state = AppState {
    pool,
    jwt: Arc::new(JwtKeys::new("test-secret")),
    cookie_domain: "localhost".into(),
    cookie_secure: false,
    app_base_url: "http://localhost:3000".into(),
    email_from: "Foundation <noreply@localhost>".into(),
    resend_api_key: None,
    bucket: None,
    cdn_base_url: "http://localhost:9000/foundation".into(),
    environment: config::Environment::Test,
    status_message: Some("Scheduled maintenance at 03:00 UTC.".into()),
    google: None,
  };
  let app = api_gateway::build_router(state);
  let resp = app.oneshot(get("/v1/api_status", None)).await.unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let body = body_json(resp).await;
  assert_eq!(body["ok"], true);
  assert_eq!(body["message"], "Scheduled maintenance at 03:00 UTC.");
}
