//! Integration tests for the account/auth queries against `foundation_test`.
//!
//! Each test runs inside a transaction that is rolled back (dropped) at the end, so the test
//! database stays clean and tests don't interfere. The pool comes from `db::test_fixtures`,
//! which HARD-GUARDS that the URL is a `*_test` database (never dev/prod).

use db_queries::users::NewUser;
use db_queries::{accounts, invitations, members, sessions, sso, users};

async fn test_pool() -> sqlx::PgPool {
  db::test_fixtures::create_test_pool().await
}

/// A unique local-part so parallel tests never collide on the `users.email` UNIQUE constraint.
fn unique_email() -> String {
  format!("t-{}@example.com", id_core::mint("t"))
}

fn new_user(email: &str, hash: Option<&str>) -> NewUser {
  NewUser {
    email: email.to_string(),
    email_clean: email.to_string(),
    email_original: email.to_string(),
    password_hash: hash.map(str::to_string),
    display_name: Some("Test User".into()),
    email_verified_at: None,
  }
}

#[tokio::test]
async fn create_user_and_find_by_email_and_id() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let email = unique_email();
  let user = users::create_user(&mut *tx, new_user(&email, Some("argon2$fakehash"))).await.unwrap();
  assert!(user.id.as_str().starts_with("user_"));
  assert_eq!(user.email, email);
  assert_eq!(user.session_version, 0);
  assert!(!user.is_staff);

  let by_email = users::find_user_by_email(&mut *tx, &email).await.unwrap().unwrap();
  assert_eq!(by_email.id, user.id);

  let by_id = users::find_user_by_id(&mut *tx, &user.id).await.unwrap().unwrap();
  assert_eq!(by_id.email, email);

  assert!(users::find_user_by_email(&mut *tx, "nobody@example.com").await.unwrap().is_none());
  // tx dropped → rolled back
}

#[tokio::test]
async fn signup_transaction_creates_user_account_and_owner_membership() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let user = users::create_user(&mut *tx, new_user(&unique_email(), Some("h"))).await.unwrap();
  let account = accounts::create_account(&mut *tx, accounts::NewAccount { name: Some("Acme"), placeholder_business_name: "Placeholder Business" }).await.unwrap();
  assert!(account.id.as_str().starts_with("acct_"));
  assert_eq!(account.status, "active");

  let mem = members::create_membership(&mut *tx, &account.id, &user.id, "owner").await.unwrap();
  assert_eq!(mem.role, "owner");

  let mems = members::list_memberships_for_user(&mut *tx, &user.id).await.unwrap();
  assert_eq!(mems.len(), 1);
  assert_eq!(mems[0].account_id, account.id);

  let found = members::find_membership(&mut *tx, &account.id, &user.id).await.unwrap().unwrap();
  assert_eq!(found.role, "owner");
}

#[tokio::test]
async fn second_owner_is_rejected_by_partial_unique_index() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let account = accounts::create_account(&mut *tx, accounts::NewAccount { name: None, placeholder_business_name: "Placeholder Business" }).await.unwrap();
  let u1 = users::create_user(&mut *tx, new_user(&unique_email(), Some("h"))).await.unwrap();
  let u2 = users::create_user(&mut *tx, new_user(&unique_email(), Some("h"))).await.unwrap();

  members::create_membership(&mut *tx, &account.id, &u1.id, "owner").await.unwrap();
  // A second owner on the same account must violate the partial unique index.
  let err = members::create_membership(&mut *tx, &account.id, &u2.id, "owner").await;
  assert!(err.is_err(), "second owner should be rejected");
}

#[tokio::test]
async fn invalid_role_is_rejected_by_check_constraint() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let account = accounts::create_account(&mut *tx, accounts::NewAccount { name: None, placeholder_business_name: "Placeholder Business" }).await.unwrap();
  let user = users::create_user(&mut *tx, new_user(&unique_email(), Some("h"))).await.unwrap();
  let err = members::create_membership(&mut *tx, &account.id, &user.id, "supervisor").await;
  assert!(err.is_err(), "role 'supervisor' should violate the CHECK constraint");
}

#[tokio::test]
async fn create_session_find_and_revoke() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let user = users::create_user(&mut *tx, new_user(&unique_email(), Some("h"))).await.unwrap();
  let expires = chrono::Utc::now() + chrono::Duration::hours(24);
  let session = sessions::create_session(
    &mut *tx,
    sessions::NewSession {
      user_id: &user.id,
      expires_at: expires,
      ip: Some("203.0.113.7"),
      user_agent: Some("test-agent"),
      device: None,
      impersonator_user_id: None,
    },
  )
  .await
  .unwrap();
  assert!(session.id.as_str().starts_with("sess_"));
  assert!(session.revoked_at.is_none());

  let live = sessions::list_live_sessions_for_user(&mut *tx, &user.id).await.unwrap();
  assert_eq!(live.len(), 1);

  let revoked = sessions::revoke_session(&mut *tx, &session.id).await.unwrap();
  assert!(revoked);
  let after = sessions::find_session_by_id(&mut *tx, &session.id).await.unwrap().unwrap();
  assert!(after.revoked_at.is_some());
  assert!(sessions::list_live_sessions_for_user(&mut *tx, &user.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn session_version_bump_increments() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let user = users::create_user(&mut *tx, new_user(&unique_email(), Some("h"))).await.unwrap();
  let sv = users::bump_session_version(&mut *tx, &user.id).await.unwrap();
  assert_eq!(sv, 1);
  let sv2 = users::bump_session_version(&mut *tx, &user.id).await.unwrap();
  assert_eq!(sv2, 2);
}

#[tokio::test]
async fn sso_connection_resolves_by_provider_sub() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let user = users::create_user(&mut *tx, new_user(&unique_email(), None)).await.unwrap();
  let sub = format!("sub-{}", id_core::mint("g"));

  let conn = sso::create_sso_connection(
    &mut *tx,
    sso::NewSsoConnection {
      user_id: &user.id,
      provider: "google",
      provider_user_id: &sub,
      provider_email: "g@example.com",
      provider_user_display_name: Some("G User"),
    },
  )
  .await
  .unwrap();
  assert!(conn.id.as_str().starts_with("sso_"));

  let found = sso::find_sso_by_provider_sub(&mut *tx, "google", &sub).await.unwrap().unwrap();
  assert_eq!(found.user_id, user.id);
  assert!(sso::find_sso_by_provider_sub(&mut *tx, "github", &sub).await.unwrap().is_none());
}

#[tokio::test]
async fn invitation_create_resolve_and_accept() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let inviter = users::create_user(&mut *tx, new_user(&unique_email(), Some("h"))).await.unwrap();
  let account = accounts::create_account(&mut *tx, accounts::NewAccount { name: Some("Team"), placeholder_business_name: "Placeholder Business" }).await.unwrap();
  members::create_membership(&mut *tx, &account.id, &inviter.id, "owner").await.unwrap();

  let code = "7F3K-QT9M";
  let inv = invitations::create_invitation(
    &mut *tx,
    invitations::NewInvitation {
      account_id: &account.id,
      invited_by_user_id: &inviter.id,
      email: "invitee@example.com",
      role: "member",
      code,
      token_hash: "deadbeef",
      expires_at: chrono::Utc::now() + chrono::Duration::days(7),
    },
  )
  .await
  .unwrap();
  assert!(inv.id.as_str().starts_with("inv_"));

  let by_code = invitations::find_invitation_by_code(&mut *tx, code).await.unwrap().unwrap();
  assert_eq!(by_code.id, inv.id);
  let by_token = invitations::find_invitation_by_token_hash(&mut *tx, "deadbeef").await.unwrap().unwrap();
  assert_eq!(by_token.id, inv.id);

  // Accept as a newly-created user → membership.
  let joiner = users::create_user(&mut *tx, new_user("invitee@example.com", Some("h"))).await.unwrap();
  let accepted = invitations::mark_invitation_accepted(&mut *tx, &inv.id, &joiner.id).await.unwrap();
  assert_eq!(accepted, 1);
  members::create_membership(&mut *tx, &account.id, &joiner.id, &by_code.role).await.unwrap();

  // Second accept is a no-op.
  let again = invitations::mark_invitation_accepted(&mut *tx, &inv.id, &joiner.id).await.unwrap();
  assert_eq!(again, 0);

  let mems = members::list_memberships_for_user(&mut *tx, &joiner.id).await.unwrap();
  assert_eq!(mems.len(), 1);
  assert_eq!(mems[0].role, "member");
}

#[tokio::test]
async fn create_account_stores_placeholder_and_null_business_name() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let account = accounts::create_account(
    &mut *tx,
    accounts::NewAccount { name: None, placeholder_business_name: "Radiant Compass" },
  )
  .await
  .unwrap();
  assert_eq!(account.placeholder_business_name, "Radiant Compass");
  assert_eq!(account.business_name, None);

  let found = accounts::find_account_by_id(&mut *tx, &account.id).await.unwrap().unwrap();
  assert_eq!(found.placeholder_business_name, "Radiant Compass");
  assert_eq!(found.business_name, None);
}

#[tokio::test]
async fn set_business_name_sets_clears_and_misses_unknown_accounts() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let account = accounts::create_account(
    &mut *tx,
    accounts::NewAccount { name: None, placeholder_business_name: "Radiant Compass" },
  )
  .await
  .unwrap();

  // Set.
  let updated = accounts::set_business_name(&mut *tx, &account.id, Some("Acme Corp")).await.unwrap().unwrap();
  assert_eq!(updated.business_name.as_deref(), Some("Acme Corp"));
  // The placeholder survives as the fallback.
  assert_eq!(updated.placeholder_business_name, "Radiant Compass");

  // Clear → back to null (clients fall back to the placeholder).
  let cleared = accounts::set_business_name(&mut *tx, &account.id, None).await.unwrap().unwrap();
  assert_eq!(cleared.business_name, None);

  // Unknown account → None, no rows touched.
  let ghost: ids::AccountId = ids::AccountId::generate();
  assert!(accounts::set_business_name(&mut *tx, &ghost, Some("X")).await.unwrap().is_none());
}

#[tokio::test]
async fn memberships_with_accounts_carry_business_names() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let user = users::create_user(&mut *tx, new_user(&unique_email(), Some("h"))).await.unwrap();
  let account = accounts::create_account(
    &mut *tx,
    accounts::NewAccount { name: None, placeholder_business_name: "Radiant Compass" },
  )
  .await
  .unwrap();
  members::create_membership(&mut *tx, &account.id, &user.id, "owner").await.unwrap();

  let rows = members::list_memberships_with_accounts(&mut *tx, &user.id).await.unwrap();
  assert_eq!(rows.len(), 1);
  assert_eq!(rows[0].business_name, None);
  assert_eq!(rows[0].placeholder_business_name, "Radiant Compass");

  accounts::set_business_name(&mut *tx, &account.id, Some("Acme Corp")).await.unwrap();
  let rows = members::list_memberships_with_accounts(&mut *tx, &user.id).await.unwrap();
  assert_eq!(rows[0].business_name.as_deref(), Some("Acme Corp"));
}
