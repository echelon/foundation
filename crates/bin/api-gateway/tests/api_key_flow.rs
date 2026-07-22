//! End-to-end API-key management (auth.md §4, revised 2026-07-21: ANY member may create and
//! delete): mint with one-time secret, list, delete, cross-account isolation, audit rows.

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
    resend_api_key: None,
    bucket: None,
    cdn_base_url: "http://localhost:9000/foundation".into(),
    environment: config::Environment::Test,
    status_message: None,
    google: None,
    github: None,
    openai: None,
    openai_webhook_secret: None,
  };
  (api_gateway::build_router(state), pool)
}

fn unique_email() -> String {
  format!("user-{}@example.com", id_core::mint("t"))
}

fn req(method: &str, uri: &str, body: Option<Value>, cookie: Option<&str>) -> Request<Body> {
  let mut b = Request::builder().method(method).uri(uri);
  if body.is_some() {
    b = b.header("content-type", "application/json");
  }
  if let Some(c) = cookie {
    b = b.header(header::COOKIE, c);
  }
  match body {
    Some(v) => b.body(Body::from(serde_json::to_vec(&v).unwrap())).unwrap(),
    None => b.body(Body::empty()).unwrap(),
  }
}

fn session_cookie(resp: &axum::response::Response) -> Option<String> {
  for v in resp.headers().get_all(header::SET_COOKIE) {
    let s = v.to_str().ok()?;
    if let Some(rest) = s.strip_prefix("foundation_session=") {
      let token = rest.split(';').next().unwrap_or("");
      if !token.is_empty() {
        return Some(format!("foundation_session={token}"));
      }
    }
  }
  None
}

async fn body_json(resp: axum::response::Response) -> Value {
  let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
  serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn signup_owner(app: &Router, pool: &db::PgPool) -> (String, ids::UserId, ids::AccountId) {
  let resp = app
    .clone()
    .oneshot(req("POST", "/v1/auth/signup", Some(json!({ "email": unique_email(), "password": "password1" })), None))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let cookie = session_cookie(&resp).unwrap();
  let user_id: ids::UserId = body_json(resp).await["user"]["id"].as_str().unwrap().parse().unwrap();
  let account_id = db_queries::members::list_memberships_for_user(pool, &user_id).await.unwrap()[0]
    .account_id
    .clone();
  (cookie, user_id, account_id)
}

/// Add a regular `member` to the account via a real invitation.
async fn join_as_member(app: &Router, owner_cookie: &str, account_id: &ids::AccountId) -> String {
  let invitee_email = unique_email();
  let inv = app
    .clone()
    .oneshot(req(
      "POST",
      &format!("/v1/accounts/{account_id}/invitations"),
      Some(json!({ "email": invitee_email, "role": "member" })),
      Some(owner_cookie),
    ))
    .await
    .unwrap();
  let code = body_json(inv).await["code"].as_str().unwrap().to_string();
  let signup = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/auth/signup",
      Some(json!({ "email": invitee_email, "password": "password1", "invitation": code })),
      None,
    ))
    .await
    .unwrap();
  session_cookie(&signup).unwrap()
}

#[tokio::test]
async fn any_member_can_mint_list_and_delete_keys_with_one_time_secret() {
  let (app, pool) = app_and_pool().await;
  let (owner_cookie, _owner_id, account_id) = signup_owner(&app, &pool).await;
  let member_cookie = join_as_member(&app, &owner_cookie, &account_id).await;

  // A REGULAR MEMBER mints a key.
  let created = app
    .clone()
    .oneshot(req(
      "POST",
      &format!("/v1/accounts/{account_id}/api_keys"),
      Some(json!({ "title": "  Render farm  ", "description": "workers pull jobs with this" })),
      Some(&member_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(created.status(), StatusCode::OK);
  let created = body_json(created).await;

  // The secret is canonical-form, shown here and only here; the stored prefix identifies it.
  let secret = created["secret"].as_str().unwrap();
  assert!(secret.starts_with("foundation_api_"), "canonical form (api_overview.md): {secret}");
  assert_eq!(secret.len(), "foundation_api_".len() + 64); // 32 random bytes, hex
  let item = &created["api_key"];
  assert_eq!(item["title"], "Render farm"); // trimmed
  assert_eq!(item["description"], "workers pull jobs with this");
  assert_eq!(item["key_prefix"], secret[..("foundation_api_".len() + 6)]);
  assert_eq!(item["last_used_at"], Value::Null);
  let key_id = item["id"].as_str().unwrap().to_string();

  // The list shows it to any member — prefix only, no secret, no hash anywhere.
  let list = app
    .clone()
    .oneshot(req("GET", &format!("/v1/accounts/{account_id}/api_keys"), None, Some(&owner_cookie)))
    .await
    .unwrap();
  assert_eq!(list.status(), StatusCode::OK);
  let list = body_json(list).await;
  let keys = list["api_keys"].as_array().unwrap();
  assert_eq!(keys.len(), 1);
  let listed = serde_json::to_string(&keys[0]).unwrap();
  assert!(!listed.contains(secret), "the full secret must never appear again");
  assert!(!listed.contains("hash"), "no hash-shaped fields in responses");

  // A DIFFERENT member (the owner here) deletes the member's key — any member may.
  let deleted = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/api_keys/{key_id}"), None, Some(&owner_cookie)))
    .await
    .unwrap();
  assert_eq!(deleted.status(), StatusCode::OK);
  let list = app
    .clone()
    .oneshot(req("GET", &format!("/v1/accounts/{account_id}/api_keys"), None, Some(&member_cookie)))
    .await
    .unwrap();
  assert_eq!(body_json(list).await["api_keys"].as_array().unwrap().len(), 0);

  // Deleting again: 404 (already revoked).
  let again = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/api_keys/{key_id}"), None, Some(&owner_cookie)))
    .await
    .unwrap();
  assert_eq!(again.status(), StatusCode::NOT_FOUND);

  // Both actions are audited.
  let audit = app
    .clone()
    .oneshot(req("GET", &format!("/v1/accounts/{account_id}/audit_logs"), None, Some(&owner_cookie)))
    .await
    .unwrap();
  let audit = body_json(audit).await;
  let events: Vec<&str> = audit["items"].as_array().unwrap().iter().map(|i| i["event"].as_str().unwrap()).collect();
  assert!(events.contains(&"api_key_created"), "events: {events:?}");
  assert!(events.contains(&"api_key_revoked"), "events: {events:?}");
}

#[tokio::test]
async fn api_keys_are_account_isolated() {
  let (app, pool) = app_and_pool().await;
  let (owner_cookie, _id, account_id) = signup_owner(&app, &pool).await;
  let (outsider_cookie, _oid, outsider_account_id) = signup_owner(&app, &pool).await;

  let created = app
    .clone()
    .oneshot(req(
      "POST",
      &format!("/v1/accounts/{account_id}/api_keys"),
      Some(json!({ "title": "Prod" })),
      Some(&owner_cookie),
    ))
    .await
    .unwrap();
  let key_id = body_json(created).await["api_key"]["id"].as_str().unwrap().to_string();

  // An outsider (owner of their OWN account) can't list this account's keys…
  let denied = app
    .clone()
    .oneshot(req("GET", &format!("/v1/accounts/{account_id}/api_keys"), None, Some(&outsider_cookie)))
    .await
    .unwrap();
  assert_eq!(denied.status(), StatusCode::NOT_FOUND);

  // …can't delete its key through this account…
  let denied = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/api_keys/{key_id}"), None, Some(&outsider_cookie)))
    .await
    .unwrap();
  assert_eq!(denied.status(), StatusCode::NOT_FOUND);

  // …and can't reach a foreign key id through their OWN account (the account scope in the
  // revoke query is the gate).
  let denied = app
    .clone()
    .oneshot(req(
      "DELETE",
      &format!("/v1/accounts/{outsider_account_id}/api_keys/{key_id}"),
      None,
      Some(&outsider_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(denied.status(), StatusCode::NOT_FOUND);

  // The key survived all of it.
  let list = app
    .clone()
    .oneshot(req("GET", &format!("/v1/accounts/{account_id}/api_keys"), None, Some(&owner_cookie)))
    .await
    .unwrap();
  assert_eq!(body_json(list).await["api_keys"].as_array().unwrap().len(), 1);

  // Blank titles are rejected.
  let blank = app
    .clone()
    .oneshot(req(
      "POST",
      &format!("/v1/accounts/{account_id}/api_keys"),
      Some(json!({ "title": "   " })),
      Some(&owner_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(blank.status(), StatusCode::BAD_REQUEST);
}
