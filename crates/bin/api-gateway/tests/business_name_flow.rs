//! End-to-end business-name tests through the real router against `foundation_test`:
//! signup mints a placeholder, owners/admins can set/clear `business_name`, members and
//! owners/admins of OTHER accounts cannot.

mod common;

use common::{app_and_pool, body_json, get, post, put, session_cookie, unique_email};

use axum::http::StatusCode;
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot


/// Sign up a fresh user; returns (cookie, user id, their owned account id).
async fn signup_owner(app: &Router, pool: &db::PgPool) -> (String, ids::UserId, ids::AccountId) {
  let resp = app
    .clone()
    .oneshot(post("/v1/auth/signup", json!({ "email": unique_email(), "password": "password1" }), None))
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

/// Add a user to `account_id` in `role` via a real invitation redeemed at signup.
async fn join_as(app: &Router, owner_cookie: &str, account_id: &ids::AccountId, role: enums::MemberRole) -> String {
  let invitee_email = unique_email();
  let inv = app
    .clone()
    .oneshot(post(
      &format!("/v1/accounts/{account_id}/invitations"),
      json!({ "email": invitee_email, "role": role }),
      Some(owner_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(inv.status(), StatusCode::OK);
  let code = body_json(inv).await["code"].as_str().unwrap().to_string();
  let signup = app
    .clone()
    .oneshot(post(
      "/v1/auth/signup",
      json!({ "email": invitee_email, "password": "password1", "invitation": code }),
      None,
    ))
    .await
    .unwrap();
  assert_eq!(signup.status(), StatusCode::CREATED);
  session_cookie(&signup).unwrap()
}

#[tokio::test]
async fn signup_mints_a_capitalized_placeholder_and_owner_sets_and_clears_business_name() {
  let (app, pool) = app_and_pool().await;
  let (owner_cookie, owner_id, account_id) = signup_owner(&app, &pool).await;

  // Signup minted a "Adjective Noun" placeholder; business_name starts null.
  let account = db_queries::accounts::find_account_by_id(&pool, &account_id).await.unwrap().unwrap();
  assert_eq!(account.business_name, None);
  let parts: Vec<&str> = account.placeholder_business_name.split(' ').collect();
  assert_eq!(parts.len(), 2, "placeholder should be 'Adjective Noun': {:?}", account.placeholder_business_name);
  for p in &parts {
    assert!(p.chars().next().unwrap().is_uppercase());
    assert!(p.chars().all(char::is_alphabetic));
  }

  // The accounts list carries both fields.
  let list = app.clone().oneshot(get("/v1/accounts", Some(&owner_cookie))).await.unwrap();
  let list = body_json(list).await;
  assert_eq!(list["accounts"][0]["business_name"], Value::Null);
  assert_eq!(list["accounts"][0]["placeholder_business_name"], account.placeholder_business_name);

  // Owner sets the name.
  let set = app
    .clone()
    .oneshot(put(
      &format!("/v1/accounts/{account_id}/business"),
      json!({ "business_name": "  Acme Corporation  " }),
      Some(&owner_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(set.status(), StatusCode::OK);
  let set = body_json(set).await;
  assert_eq!(set["business_name"], "Acme Corporation"); // trimmed
  assert_eq!(set["placeholder_business_name"], account.placeholder_business_name); // untouched

  // Clearing (empty string) → null; the placeholder is still there as the fallback.
  let cleared = app
    .clone()
    .oneshot(put(
      &format!("/v1/accounts/{account_id}/business"),
      json!({ "business_name": "" }),
      Some(&owner_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(cleared.status(), StatusCode::OK);
  let cleared = body_json(cleared).await;
  assert_eq!(cleared["business_name"], Value::Null);
  assert_eq!(cleared["placeholder_business_name"], account.placeholder_business_name);

  // Both changes are audited on the account trail with WHO and before/after values.
  // Newest first: [0] = the clear, [1] = the set.
  let audit = app
    .clone()
    .oneshot(get(&format!("/v1/accounts/{account_id}/audit_logs"), Some(&owner_cookie)))
    .await
    .unwrap();
  let audit = body_json(audit).await;
  let changes: Vec<&Value> = audit["items"]
    .as_array()
    .unwrap()
    .iter()
    .filter(|i| i["event"] == "business_name_changed")
    .collect();
  assert_eq!(changes.len(), 2, "expected the set + the clear: {audit:#?}");
  for change in &changes {
    assert_eq!(change["actor_user_id"], owner_id.to_string(), "audit must record who changed it");
  }
  assert_eq!(changes[0]["data"]["from"], "Acme Corporation");
  assert_eq!(changes[0]["data"]["to"], Value::Null);
  assert_eq!(changes[1]["data"]["from"], Value::Null);
  assert_eq!(changes[1]["data"]["to"], "Acme Corporation");
}

#[tokio::test]
async fn admins_can_set_business_name_but_members_cannot() {
  let (app, pool) = app_and_pool().await;
  let (owner_cookie, _owner_id, account_id) = signup_owner(&app, &pool).await;
  let admin_cookie = join_as(&app, &owner_cookie, &account_id, enums::MemberRole::Admin).await;
  let member_cookie = join_as(&app, &owner_cookie, &account_id, enums::MemberRole::Member).await;

  // Admin: allowed.
  let by_admin = app
    .clone()
    .oneshot(put(
      &format!("/v1/accounts/{account_id}/business"),
      json!({ "business_name": "Set By Admin" }),
      Some(&admin_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(by_admin.status(), StatusCode::OK);

  // Regular member: 403, and the name is untouched.
  let by_member = app
    .clone()
    .oneshot(put(
      &format!("/v1/accounts/{account_id}/business"),
      json!({ "business_name": "Set By Member" }),
      Some(&member_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(by_member.status(), StatusCode::FORBIDDEN);
  let account = db_queries::accounts::find_account_by_id(&pool, &account_id).await.unwrap().unwrap();
  assert_eq!(account.business_name.as_deref(), Some("Set By Admin"));
}

#[tokio::test]
async fn owner_of_a_different_account_cannot_set_this_ones_business_name() {
  let (app, pool) = app_and_pool().await;
  let (_target_owner_cookie, _id, target_account_id) = signup_owner(&app, &pool).await;
  // A perfectly legitimate owner (and thus admin-tier) — of a DIFFERENT account.
  let (other_owner_cookie, _other_id, other_account_id) = signup_owner(&app, &pool).await;
  assert_ne!(target_account_id, other_account_id);

  // Being an owner somewhere does not authorize editing here: same 404 an unknown account
  // gets (no existence leak).
  let cross = app
    .clone()
    .oneshot(put(
      &format!("/v1/accounts/{target_account_id}/business"),
      json!({ "business_name": "Hijacked" }),
      Some(&other_owner_cookie),
    ))
    .await
    .unwrap();
  assert_eq!(cross.status(), StatusCode::NOT_FOUND);
  let target = db_queries::accounts::find_account_by_id(&pool, &target_account_id).await.unwrap().unwrap();
  assert_eq!(target.business_name, None, "cross-account write must not land");

  // Unauthenticated: 401.
  let anon = app
    .clone()
    .oneshot(put(
      &format!("/v1/accounts/{target_account_id}/business"),
      json!({ "business_name": "Anon" }),
      None,
    ))
    .await
    .unwrap();
  assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);
}
