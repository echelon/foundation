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

#[tokio::test]
async fn sso_login_creates_user_then_logs_in_on_repeat() {
  let (app, _pool) = app_and_pool().await;
  let email = unique_email();
  let sub = format!("sub-{}", id_core::mint("g"));

  let first = app
    .clone()
    .oneshot(post("/v1/auth/oauth/google/callback", json!({ "provider_user_id": sub, "email": email, "display_name": "SSO User" }), None))
    .await
    .unwrap();
  assert_eq!(first.status(), StatusCode::OK);
  assert!(session_cookie(&first).is_some());

  // Same (provider, sub) again → logs into the same user.
  let second = app
    .oneshot(post("/v1/auth/oauth/google/callback", json!({ "provider_user_id": sub, "email": email, "display_name": "SSO User" }), None))
    .await
    .unwrap();
  assert_eq!(second.status(), StatusCode::OK);
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
  };
  let app = api_gateway::build_router(state);
  let resp = app.oneshot(get("/v1/api_status", None)).await.unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let body = body_json(resp).await;
  assert_eq!(body["ok"], true);
  assert_eq!(body["message"], "Scheduled maintenance at 03:00 UTC.");
}
