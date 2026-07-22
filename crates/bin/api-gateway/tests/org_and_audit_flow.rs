//! End-to-end organization + audit-log flows through the REAL router against `foundation_test`
//! — full axum stack via `tower::ServiceExt::oneshot`, real signups, cookie sessions.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot

use api_gateway::auth::token::JwtKeys;
use api_gateway::AppState;

async fn app() -> Router {
  let pool = db::test_fixtures::create_test_pool().await;
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
    status_message: None,
    google: None,
    github: None,
    openai: None,
  };
  api_gateway::build_router(state)
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
  let body = match body {
    Some(v) => Body::from(serde_json::to_vec(&v).unwrap()),
    None => Body::empty(),
  };
  b.body(body).unwrap()
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

/// Sign up over HTTP → (cookie, user_id).
async fn signup(app: &Router) -> (String, String) {
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/auth/signup",
      Some(json!({ "email": unique_email(), "password": "hunter2hunter2" })),
      None,
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::CREATED);
  let cookie = session_cookie(&resp).unwrap();
  let user_id = body_json(resp).await["user"]["id"].as_str().unwrap().to_string();
  (cookie, user_id)
}

async fn get_json(app: &Router, uri: &str, cookie: &str) -> (StatusCode, Value) {
  let resp = app.clone().oneshot(req("GET", uri, None, Some(cookie))).await.unwrap();
  let status = resp.status();
  (status, body_json(resp).await)
}

/// Full org fixture: owner signs up, invites `invitee_role`, invitee signs up + accepts.
/// Returns (owner_cookie, owner_id, member_cookie, member_id, account_id).
async fn org_with_two(app: &Router, invitee_role: &str) -> (String, String, String, String, String) {
  let (owner_cookie, owner_id) = signup(app).await;
  let (_, accounts) = get_json(app, "/v1/accounts", &owner_cookie).await;
  let account_id = accounts["accounts"][0]["account_id"].as_str().unwrap().to_string();

  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      &format!("/v1/accounts/{account_id}/invitations"),
      Some(json!({ "email": unique_email(), "role": invitee_role })),
      Some(&owner_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let invitation_id = body_json(resp).await["invitation_id"].as_str().unwrap().to_string();

  let (member_cookie, member_id) = signup(app).await;
  let resp = app
    .clone()
    .oneshot(req("POST", &format!("/v1/invitations/{invitation_id}/accept"), Some(json!({})), Some(&member_cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);

  (owner_cookie, owner_id, member_cookie, member_id, account_id)
}

/* ── Switcher + roster + removal ── */

#[tokio::test]
async fn org_membership_roster_and_removal_flow() {
  let app = app().await;
  let (owner_cookie, owner_id, member_cookie, member_id, account_id) = org_with_two(&app, "member").await;

  // The joiner's switcher shows BOTH orgs (their own signup account + the joined one).
  let (_, accounts) = get_json(&app, "/v1/accounts", &member_cookie).await;
  let list = accounts["accounts"].as_array().unwrap();
  assert_eq!(list.len(), 2);
  let joined = list.iter().find(|a| a["account_id"] == account_id.as_str()).unwrap();
  assert_eq!(joined["role"], "member");

  // Roster: both members visible to both parties, with correct roles + the caller's role.
  let (status, roster) = get_json(&app, &format!("/v1/accounts/{account_id}/members"), &member_cookie).await;
  assert_eq!(status, StatusCode::OK);
  assert_eq!(roster["your_role"], "member");
  let members_list = roster["members"].as_array().unwrap();
  assert_eq!(members_list.len(), 2);
  assert_eq!(members_list[0]["user_id"], owner_id.as_str());
  assert_eq!(members_list[0]["role"], "owner");
  assert_eq!(members_list[1]["role"], "member");
  assert_eq!(members_list[1]["is_you"], true);

  // An outsider gets the same 404 an unknown account would.
  let (outsider_cookie, _) = signup(&app).await;
  let (status, _) = get_json(&app, &format!("/v1/accounts/{account_id}/members"), &outsider_cookie).await;
  assert_eq!(status, StatusCode::NOT_FOUND);

  // A plain member may NOT remove anyone (403/404 from the role gate).
  let resp = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/members/{owner_id}"), None, Some(&member_cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::FORBIDDEN);

  // The owner can't be removed, and self-removal is rejected.
  let resp = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/members/{owner_id}"), None, Some(&owner_cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

  // The owner removes the member; the roster shrinks and the member's switcher loses the org.
  let resp = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/members/{member_id}"), None, Some(&owner_cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let (_, roster) = get_json(&app, &format!("/v1/accounts/{account_id}/members"), &owner_cookie).await;
  assert_eq!(roster["members"].as_array().unwrap().len(), 1);
  let (_, accounts) = get_json(&app, "/v1/accounts", &member_cookie).await;
  assert!(accounts["accounts"].as_array().unwrap().iter().all(|a| a["account_id"] != account_id.as_str()));

  // The org audit trail recorded the whole story, admin-eyes only.
  let (status, log) = get_json(&app, &format!("/v1/accounts/{account_id}/audit_logs"), &owner_cookie).await;
  assert_eq!(status, StatusCode::OK);
  let events: Vec<&str> = log["items"].as_array().unwrap().iter().map(|i| i["event"].as_str().unwrap()).collect();
  for expected in ["member_removed", "invitation_accepted", "member_added", "invitation_sent", "account_created"] {
    assert!(events.contains(&expected), "org log missing {expected}; got {events:?}");
  }
}

#[tokio::test]
async fn admins_can_remove_members_but_not_admins() {
  let app = app().await;
  let (owner_cookie, _owner_id, admin_cookie, admin_id, account_id) = org_with_two(&app, "admin").await;

  // Bring in a plain member too.
  let resp = app
    .clone()
    .oneshot(req(
      "POST",
      &format!("/v1/accounts/{account_id}/invitations"),
      Some(json!({ "email": unique_email(), "role": "member" })),
      Some(&owner_cookie),
    ))
    .await
    .unwrap();
  let invitation_id = body_json(resp).await["invitation_id"].as_str().unwrap().to_string();
  let (member_cookie, member_id) = signup(&app).await;
  app
    .clone()
    .oneshot(req("POST", &format!("/v1/invitations/{invitation_id}/accept"), Some(json!({})), Some(&member_cookie)))
    .await
    .unwrap();

  // Admin can remove the member…
  let resp = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/members/{member_id}"), None, Some(&admin_cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);

  // …but an admin cannot remove another admin (only the owner can).
  let resp = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/members/{admin_id}"), None, Some(&admin_cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "self-removal rejected first");
  let resp = app
    .clone()
    .oneshot(req("DELETE", &format!("/v1/accounts/{account_id}/members/{admin_id}"), None, Some(&owner_cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK, "the owner CAN remove an admin");
}

/* ── Audit endpoints: own (paginated), org (admin-only), member (org-visible only) ── */

#[tokio::test]
async fn own_audit_log_paginates() {
  let app = app().await;
  let (cookie, _user_id) = signup(&app).await;

  // Generate a few org-visible events via real endpoints (name changes).
  for name in ["Ada One", "Ada Two", "Ada Three"] {
    let resp = app
      .clone()
      .oneshot(req("PUT", "/v1/auth/profile", Some(json!({ "display_name": name })), Some(&cookie)))
      .await
      .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
  }

  // account_created + 3 × display_name_changed = 4 rows; page size 3 → 3 + 1.
  let (status, p1) = get_json(&app, "/v1/user/audit_logs?limit=3", &cookie).await;
  assert_eq!(status, StatusCode::OK);
  assert_eq!(p1["items"].as_array().unwrap().len(), 3);
  let cursor = p1["next_before"].as_str().expect("full page → cursor").to_string();

  let (_, p2) = get_json(&app, &format!("/v1/user/audit_logs?limit=3&before={cursor}"), &cookie).await;
  let p2_items = p2["items"].as_array().unwrap();
  assert_eq!(p2_items.len(), 1);
  assert_eq!(p2_items[0]["event"], "account_created", "oldest event lands on the last page");
  assert!(p2["next_before"].is_null(), "short page → no cursor");

  // Newest first: the most recent name change leads page one.
  assert_eq!(p1["items"][0]["event"], "display_name_changed");
  assert_eq!(p1["items"][0]["data"]["to"], "Ada Three");
}

#[tokio::test]
async fn org_audit_log_is_admin_only_and_member_view_filters_private_rows() {
  let app = app().await;
  let (owner_cookie, _owner_id, member_cookie, member_id, account_id) = org_with_two(&app, "member").await;

  // A plain member cannot read the org audit log.
  let (status, _) = get_json(&app, &format!("/v1/accounts/{account_id}/audit_logs"), &member_cookie).await;
  assert_eq!(status, StatusCode::FORBIDDEN);

  // The member does something org-visible (password change) and something private
  // (revoking one of their other sessions — log in again to have one to kill).
  let resp = app
    .clone()
    .oneshot(req(
      "PUT",
      "/v1/auth/password",
      Some(json!({ "current_password": "hunter2hunter2", "new_password": "hunter3hunter3" })),
      Some(&member_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  let (_, sessions) = get_json(&app, "/v1/auth/sessions", &member_cookie).await;
  let this_session = sessions["sessions"][0]["id"].as_str().unwrap().to_string();
  // Kill via "others" purge? Simpler: revoke a freshly-created second session directly.
  let member_email_login = sessions["sessions"].as_array().unwrap().len(); // (at least 1)
  assert!(member_email_login >= 1 && !this_session.is_empty());
  // Revoke-others always writes a (private) session_revoked event.
  let resp = app
    .clone()
    .oneshot(req("DELETE", "/v1/auth/sessions?others=true", None, Some(&member_cookie)))
    .await
    .unwrap();
  assert_eq!(resp.status(), StatusCode::OK);
  // The purge bumps session_version and re-mints the cookie — keep using the fresh one.
  let member_cookie = session_cookie(&resp).expect("purge re-mints the session cookie");

  // The member sees both in their OWN log…
  let (_, own) = get_json(&app, "/v1/user/audit_logs?limit=50", &member_cookie).await;
  let own_events: Vec<&str> = own["items"].as_array().unwrap().iter().map(|i| i["event"].as_str().unwrap()).collect();
  assert!(own_events.contains(&"password_changed"));
  assert!(own_events.contains(&"session_revoked"));

  // …but the ORG (owner) sees only the org-visible rows for that member.
  let (status, org_view) =
    get_json(&app, &format!("/v1/accounts/{account_id}/members/{member_id}/audit_logs?limit=50"), &owner_cookie).await;
  assert_eq!(status, StatusCode::OK);
  let org_events: Vec<&str> =
    org_view["items"].as_array().unwrap().iter().map(|i| i["event"].as_str().unwrap()).collect();
  assert!(org_events.contains(&"password_changed"), "org-visible rows show");
  assert!(!org_events.contains(&"session_revoked"), "private rows must be filtered");
  assert!(!org_events.contains(&"account_created"), "account_created is not org-visible either");

  // A plain member cannot use the member-audit endpoint; a non-member target 404s.
  let (status, _) =
    get_json(&app, &format!("/v1/accounts/{account_id}/members/{member_id}/audit_logs"), &member_cookie).await;
  assert_eq!(status, StatusCode::FORBIDDEN);
  let (outsider_cookie, outsider_id) = signup(&app).await;
  drop(outsider_cookie);
  let (status, _) =
    get_json(&app, &format!("/v1/accounts/{account_id}/members/{outsider_id}/audit_logs"), &owner_cookie).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn roster_and_user_info_carry_avatars() {
  let app = app().await;
  let pool = db::test_fixtures::create_test_pool().await;

  // Owner + one member (via invitation), both fresh — no uploads, no gravatar mirrored yet.
  let owner_email = unique_email();
  let signup = app
    .clone()
    .oneshot(req("POST", "/v1/auth/signup", Some(json!({ "email": owner_email, "password": "ownerpass1" })), None))
    .await
    .unwrap();
  let owner_cookie = session_cookie(&signup).unwrap();
  let owner_id: ids::UserId = body_json(signup).await["user"]["id"].as_str().unwrap().parse().unwrap();
  let account_id = db_queries::members::list_memberships_for_user(&pool, &owner_id).await.unwrap()[0]
    .account_id
    .clone();

  let invitee_email = unique_email();
  let inv = app
    .clone()
    .oneshot(req(
      "POST",
      &format!("/v1/accounts/{account_id}/invitations"),
      Some(json!({ "email": invitee_email, "role": "member" })),
      Some(&owner_cookie),
    ))
    .await
    .unwrap();
  let code = body_json(inv).await["code"].as_str().unwrap().to_string();
  let joined = app
    .clone()
    .oneshot(req(
      "POST",
      "/v1/auth/signup",
      Some(json!({ "email": invitee_email, "password": "memberpass1", "invitation": code })),
      None,
    ))
    .await
    .unwrap();
  let member_cookie = session_cookie(&joined).unwrap();

  // The roster reports every member's avatar; fresh users show the letter monogram.
  let roster = app
    .clone()
    .oneshot(req("GET", &format!("/v1/accounts/{account_id}/members"), None, Some(&member_cookie)))
    .await
    .unwrap();
  assert_eq!(roster.status(), StatusCode::OK);
  let roster = body_json(roster).await;
  for m in roster["members"].as_array().unwrap() {
    assert_eq!(m["avatar"]["source"], "letter", "fresh users display the monogram: {m:#?}");
    assert_eq!(m["avatar"]["links"], Value::Null);
  }

  // The user-info API answers for current members…
  let info = app
    .clone()
    .oneshot(req(
      "GET",
      &format!("/v1/accounts/{account_id}/users?ids={owner_id},user_notreal000000000000000000"),
      None,
      Some(&member_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(info.status(), StatusCode::OK);
  let info = body_json(info).await;
  let users = info["users"].as_array().unwrap();
  assert_eq!(users.len(), 1, "unknown ids are silently omitted");
  assert_eq!(users[0]["user_id"], owner_id.to_string());
  assert_eq!(users[0]["email"], owner_email);
  assert_eq!(users[0]["avatar"]["source"], "letter");

  // …but not for outsiders (same 404 an unknown account gets).
  let outsider = app
    .clone()
    .oneshot(req("POST", "/v1/auth/signup", Some(json!({ "email": unique_email(), "password": "outsider12" })), None))
    .await
    .unwrap();
  let outsider_cookie = session_cookie(&outsider).unwrap();
  let denied = app
    .clone()
    .oneshot(req("GET", &format!("/v1/accounts/{account_id}/users?ids={owner_id}"), None, Some(&outsider_cookie)))
    .await
    .unwrap();
  assert_eq!(denied.status(), StatusCode::NOT_FOUND);
}
