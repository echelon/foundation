//! Integration tests for the organization-membership and audit-log queries against
//! `foundation_test` (tx-rollback harness, hard-guarded test db).

use db_queries::users::NewUser;
use db_queries::{account_audit_logs, accounts, members, user_audit_logs, users};
use enums::AuditEvent;

async fn test_pool() -> sqlx::PgPool {
  db::test_fixtures::create_test_pool().await
}

fn unique_email() -> String {
  format!("t-{}@example.com", id_core::mint("t"))
}

fn new_user(email: &str) -> NewUser {
  NewUser {
    email: email.to_string(),
    email_clean: email.to_string(),
    email_original: email.to_string(),
    password_hash: Some("argon2$fakehash".into()),
    display_name: Some("Org Tester".into()),
    email_verified_at: None,
  }
}

async fn user_event(
  tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
  user_id: &ids::UserId,
  event: AuditEvent,
) -> db_queries::UserAuditLog {
  user_audit_logs::create_user_audit_log(
    &mut **tx,
    user_audit_logs::NewUserAuditLog {
      user_id,
      actor_user_id: Some(user_id),
      event,
      target_type: None,
      target_id: None,
      data: None,
      ip: None,
    },
  )
  .await
  .unwrap()
}

#[tokio::test]
async fn memberships_with_accounts_joins_names_and_roles() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let user = users::create_user(&mut *tx, new_user(&unique_email())).await.unwrap();
  let acme = accounts::create_account(&mut *tx, accounts::NewAccount { name: Some("Acme"), placeholder_business_name: "Placeholder Business" }).await.unwrap();
  let globex = accounts::create_account(&mut *tx, accounts::NewAccount { name: Some("Globex"), placeholder_business_name: "Placeholder Business" }).await.unwrap();
  members::create_membership(&mut *tx, &acme.id, &user.id, enums::MemberRole::Owner).await.unwrap();
  members::create_membership(&mut *tx, &globex.id, &user.id, enums::MemberRole::Member).await.unwrap();

  let list = members::list_memberships_with_accounts(&mut *tx, &user.id).await.unwrap();
  assert_eq!(list.len(), 2);
  // Oldest membership first — the switcher's stable order.
  assert_eq!(list[0].account_id, acme.id);
  assert_eq!(list[0].account_name.as_deref(), Some("Acme"));
  assert_eq!(list[0].role, enums::MemberRole::Owner);
  assert_eq!(list[1].account_name.as_deref(), Some("Globex"));
  assert_eq!(list[1].role, enums::MemberRole::Member);
}

#[tokio::test]
async fn roster_join_and_member_removal() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  let owner = users::create_user(&mut *tx, new_user(&unique_email())).await.unwrap();
  let member = users::create_user(&mut *tx, new_user(&unique_email())).await.unwrap();
  let org = accounts::create_account(&mut *tx, accounts::NewAccount { name: Some("Team"), placeholder_business_name: "Placeholder Business" }).await.unwrap();
  members::create_membership(&mut *tx, &org.id, &owner.id, enums::MemberRole::Owner).await.unwrap();
  members::create_membership(&mut *tx, &org.id, &member.id, enums::MemberRole::Member).await.unwrap();

  let roster = members::list_members_of_account(&mut *tx, &org.id).await.unwrap();
  assert_eq!(roster.len(), 2);
  assert_eq!(roster[0].user_id, owner.id, "owner (oldest membership) leads the roster");
  assert_eq!(roster[0].role, enums::MemberRole::Owner);
  assert_eq!(roster[1].email, member.email);

  let removed = members::delete_membership(&mut *tx, &org.id, &member.id).await.unwrap();
  assert_eq!(removed, 1);
  assert_eq!(members::list_members_of_account(&mut *tx, &org.id).await.unwrap().len(), 1);
  // Removing a non-member is a no-op, not an error.
  assert_eq!(members::delete_membership(&mut *tx, &org.id, &member.id).await.unwrap(), 0);
}

#[tokio::test]
async fn user_audit_log_pagination_pages_through_without_gaps() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let user = users::create_user(&mut *tx, new_user(&unique_email())).await.unwrap();

  for _ in 0..5 {
    user_event(&mut tx, &user.id, AuditEvent::PasswordChanged).await;
  }

  // Page size 2: 2 + 2 + 1, newest first, no overlaps.
  let p1 = user_audit_logs::list_user_audit_logs(&mut *tx, &user.id, None, 2).await.unwrap();
  assert_eq!(p1.len(), 2);
  let p2 = user_audit_logs::list_user_audit_logs(&mut *tx, &user.id, Some(&p1[1].id), 2).await.unwrap();
  assert_eq!(p2.len(), 2);
  let p3 = user_audit_logs::list_user_audit_logs(&mut *tx, &user.id, Some(&p2[1].id), 2).await.unwrap();
  assert_eq!(p3.len(), 1);

  let mut all: Vec<String> = [&p1[..], &p2[..], &p3[..]].concat().iter().map(|r| r.id.to_string()).collect();
  let total = all.len();
  all.dedup();
  assert_eq!(all.len(), total, "pages must not overlap");
  assert_eq!(total, 5);
}

#[tokio::test]
async fn org_visible_listing_hides_private_events() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let user = users::create_user(&mut *tx, new_user(&unique_email())).await.unwrap();

  user_event(&mut tx, &user.id, AuditEvent::PasswordChanged).await; // org-visible
  user_event(&mut tx, &user.id, AuditEvent::SessionRevoked).await; // user-private
  user_event(&mut tx, &user.id, AuditEvent::EmailChanged).await; // org-visible

  let own = user_audit_logs::list_user_audit_logs(&mut *tx, &user.id, None, 10).await.unwrap();
  assert_eq!(own.len(), 3, "the user sees everything");

  let org_view = user_audit_logs::list_org_visible_user_audit_logs(&mut *tx, &user.id, None, 10).await.unwrap();
  assert_eq!(org_view.len(), 2, "the org sees only org-visible rows");
  assert!(org_view.iter().all(|r| r.is_org_visible));
  assert!(org_view.iter().all(|r| r.event != "session_revoked"));
}

#[tokio::test]
async fn account_audit_log_pagination() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let owner = users::create_user(&mut *tx, new_user(&unique_email())).await.unwrap();
  let org = accounts::create_account(&mut *tx, accounts::NewAccount { name: Some("Audited Org"), placeholder_business_name: "Placeholder Business" }).await.unwrap();

  for _ in 0..3 {
    account_audit_logs::create_account_audit_log(
      &mut *tx,
      account_audit_logs::NewAccountAuditLog {
        account_id: &org.id,
        actor_user_id: Some(&owner.id),
        user_id: None,
        event: AuditEvent::ApiKeyCreated,
        target_type: None,
        target_id: None,
        data: None,
        ip: None,
      },
    )
    .await
    .unwrap();
  }

  let p1 = account_audit_logs::list_account_audit_logs(&mut *tx, &org.id, None, 2).await.unwrap();
  assert_eq!(p1.len(), 2);
  let p2 = account_audit_logs::list_account_audit_logs(&mut *tx, &org.id, Some(&p1[1].id), 2).await.unwrap();
  assert_eq!(p2.len(), 1);
  assert!(p1.iter().all(|r| r.event == "api_key_created"));
}
