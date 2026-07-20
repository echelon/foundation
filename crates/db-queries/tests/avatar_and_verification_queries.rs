//! Integration tests for the avatar, email-change, and email-verification queries against
//! `foundation_test`. Same harness as `accounts_queries.rs`: every test runs inside a
//! transaction that is dropped (rolled back) at the end; `db::test_fixtures` hard-guards the
//! URL is a `*_test` database.

use db_queries::users::NewUser;
use db_queries::{email_changes, email_sends, email_verifications, user_audit_logs, user_avatar_files, user_avatars, users};
use enums::{AuditEvent, AvatarFileSource, AvatarSource, EmailStatus, EmailTemplate};

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
    display_name: Some("Test User".into()),
    email_verified_at: None,
  }
}

/// Create a user + their avatar row (as the signup transaction does).
async fn user_with_avatar<'e>(
  tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> (db_queries::User, db_queries::UserAvatar) {
  let user = users::create_user(&mut **tx, new_user(&unique_email())).await.unwrap();
  let avatar = user_avatars::create_user_avatar(
    &mut **tx,
    user_avatars::NewUserAvatar {
      user_id: &user.id,
      gravatar_hash: "deadbeef",
      generated_seed: user.id.as_str(),
      email_first_letter: Some("T"),
      email_last_letter: Some("T"),
    },
  )
  .await
  .unwrap();
  (user, avatar)
}

/// Plant an avatar file row (a mirrored gravatar or an uploaded custom image).
async fn plant_file(
  tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
  user_id: &ids::UserId,
  source: AvatarFileSource,
  original: Option<&ids::UserAvatarFileId>,
) -> db_queries::UserAvatarFile {
  // 32 unique chars (a ULID body is 26; pad with its own head) — shaped like a real bucket_hash.
  let ulid = id_core::mint("h");
  let bucket_hash = format!("{}{}", &ulid[2..28], &ulid[2..8]);
  user_avatar_files::create_user_avatar_file(
    &mut **tx,
    user_avatar_files::NewUserAvatarFile {
      user_id,
      original_user_avatar_file_id: original,
      avatar_source: source,
      bucket_hash: &bucket_hash,
      content_hash: "cafebabe",
      bucket_suffix: None,
      bucket_extension: ".png",
      width: Some(512),
      height: Some(512),
      bytes: 1234,
    },
  )
  .await
  .unwrap()
}

/* ── Gravatar: mirror, change (re-poll), clear ── */

#[tokio::test]
async fn changing_gravatar_replaces_the_mirror_via_repoll() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let (user, avatar) = user_with_avatar(&mut tx).await;

  // Fresh avatar: unpolled, auto preference.
  assert_eq!(avatar.preferred_source, AvatarSource::NotYetSet);
  assert!(avatar.gravatar_checked_at.is_none());

  // The job mirrors gravatar A.
  let file_a = plant_file(&mut tx, &user.id, AvatarFileSource::GravatarMirror, None).await;
  user_avatars::set_gravatar_mirrored(&mut *tx, &user.id, &file_a.id).await.unwrap();
  let a = user_avatars::find_user_avatar_by_user(&mut *tx, &user.id).await.unwrap().unwrap();
  assert_eq!(a.gravatar_file_id.as_ref(), Some(&file_a.id));
  assert!(a.gravatar_checked_at.is_some());
  assert_eq!(a.preferred_source, AvatarSource::NotYetSet, "mirroring must not steal the user's choice");

  // The user asks for a refresh → back in the unpolled set → the job mirrors gravatar B.
  user_avatars::request_gravatar_repoll(&mut *tx, &user.id).await.unwrap();
  let repolled = user_avatars::find_user_avatar_by_user(&mut *tx, &user.id).await.unwrap().unwrap();
  assert!(repolled.gravatar_checked_at.is_none(), "repoll request must clear the probe stamp");

  let file_b = plant_file(&mut tx, &user.id, AvatarFileSource::GravatarMirror, None).await;
  user_avatars::set_gravatar_mirrored(&mut *tx, &user.id, &file_b.id).await.unwrap();
  let b = user_avatars::find_user_avatar_by_user(&mut *tx, &user.id).await.unwrap().unwrap();
  assert_eq!(b.gravatar_file_id.as_ref(), Some(&file_b.id), "the mirror should point at the NEW file");

  // A re-poll that finds nothing (gravatar deleted remotely) clears the mirror.
  user_avatars::mark_gravatar_checked(&mut *tx, &user.id, false).await.unwrap();
  let gone = user_avatars::find_user_avatar_by_user(&mut *tx, &user.id).await.unwrap().unwrap();
  assert!(gone.gravatar_file_id.is_none());
  assert!(gone.gravatar_checked_at.is_some());
}

#[tokio::test]
async fn dismissing_gravatar_keeps_the_mirror_and_removing_clears_it() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let (user, _) = user_with_avatar(&mut tx).await;

  let mirror = plant_file(&mut tx, &user.id, AvatarFileSource::GravatarMirror, None).await;
  user_avatars::set_gravatar_mirrored(&mut *tx, &user.id, &mirror.id).await.unwrap();

  // "Dismiss" in the preference model: prefer the monogram; the mirror is KEPT.
  user_avatars::set_preferred_source(&mut *tx, &user.id, AvatarSource::Letter).await.unwrap();
  let dismissed = user_avatars::find_user_avatar_by_user(&mut *tx, &user.id).await.unwrap().unwrap();
  assert_eq!(dismissed.preferred_source, AvatarSource::Letter);
  assert_eq!(dismissed.gravatar_file_id.as_ref(), Some(&mirror.id), "dismissal keeps the mirror");

  // Choosing it back works because the mirror survived.
  user_avatars::set_preferred_source(&mut *tx, &user.id, AvatarSource::Gravatar).await.unwrap();

  // REMOVING actually clears the mirror; a `gravatar` preference falls back to auto.
  user_avatars::remove_gravatar(&mut *tx, &user.id).await.unwrap();
  let removed = user_avatars::find_user_avatar_by_user(&mut *tx, &user.id).await.unwrap().unwrap();
  assert!(removed.gravatar_file_id.is_none());
  assert_eq!(removed.preferred_source, AvatarSource::NotYetSet);
  assert!(removed.gravatar_checked_at.is_some(), "removal must NOT trigger an automatic re-poll");
}

/* ── Custom uploads: set, dismiss ── */

#[tokio::test]
async fn setting_uploaded_avatar_prefers_it_and_keeps_lineage() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let (user, _) = user_with_avatar(&mut tx).await;

  // Original + baked crop (lineage), as the upload endpoint records them.
  let original = plant_file(&mut tx, &user.id, AvatarFileSource::CustomUpload, None).await;
  let baked = plant_file(&mut tx, &user.id, AvatarFileSource::CustomUpload, Some(&original.id)).await;
  assert_eq!(baked.original_user_avatar_file_id.as_ref(), Some(&original.id));

  user_avatars::set_custom_avatar(&mut *tx, &user.id, &baked.id).await.unwrap();
  let a = user_avatars::find_user_avatar_by_user(&mut *tx, &user.id).await.unwrap().unwrap();
  assert_eq!(a.custom_file_id.as_ref(), Some(&baked.id));
  assert_eq!(a.preferred_source, AvatarSource::Custom, "uploading implies preferring it");

  let fetched = user_avatar_files::find_user_avatar_file_by_id(&mut *tx, &baked.id).await.unwrap().unwrap();
  assert_eq!(fetched.avatar_source, AvatarFileSource::CustomUpload);
}

#[tokio::test]
async fn dismissing_uploaded_avatar_falls_back_to_auto() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let (user, _) = user_with_avatar(&mut tx).await;

  let baked = plant_file(&mut tx, &user.id, AvatarFileSource::CustomUpload, None).await;
  user_avatars::set_custom_avatar(&mut *tx, &user.id, &baked.id).await.unwrap();

  user_avatars::remove_custom_avatar(&mut *tx, &user.id).await.unwrap();
  let a = user_avatars::find_user_avatar_by_user(&mut *tx, &user.id).await.unwrap().unwrap();
  assert!(a.custom_file_id.is_none(), "the pointer clears (the immutable file row is kept)");
  assert_eq!(a.preferred_source, AvatarSource::NotYetSet, "back to auto-pick");
}

/* ── Email change: verified state resets + logs written ── */

#[tokio::test]
async fn changing_email_resets_verified_state_and_writes_the_logs() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();

  // A VERIFIED user with an outstanding verification token.
  let email = unique_email();
  let mut nu = new_user(&email);
  nu.email_verified_at = Some(chrono::Utc::now());
  let user = users::create_user(&mut *tx, nu).await.unwrap();
  assert!(user.email_verified_at.is_some());
  email_verifications::create_email_verification(
    &mut *tx,
    email_verifications::NewEmailVerification {
      user_id: &user.id,
      email: &user.email,
      token_hash: "aaaa",
      expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
    },
  )
  .await
  .unwrap();

  // The five atomic steps of an email change (accounts.md §2), at the query level.
  let new_email = unique_email();
  let change = email_changes::create_email_change(
    &mut *tx,
    email_changes::NewEmailChange {
      user_id: &user.id,
      old_email: &user.email,
      old_email_clean: &user.email_clean,
      old_email_original: &user.email_original,
      new_email: &new_email,
      new_email_clean: &new_email,
      new_email_original: &new_email,
      change_source: "user",
    },
  )
  .await
  .unwrap();
  assert_eq!(change.old_email, email);
  assert_eq!(change.new_email, new_email);

  let audit = user_audit_logs::create_user_audit_log(
    &mut *tx,
    user_audit_logs::NewUserAuditLog {
      user_id: &user.id,
      actor_user_id: Some(&user.id),
      event: AuditEvent::EmailChanged,
      target_type: None,
      target_id: None,
      data: Some(serde_json::json!({ "old_email": user.email, "new_email": new_email })),
      ip: None,
    },
  )
  .await
  .unwrap();
  assert_eq!(audit.event, "email_changed");
  assert!(audit.is_org_visible, "email_changed is org-visible (audit_logs.md §3)");

  users::update_email(&mut *tx, &user.id, &new_email, &new_email, &new_email).await.unwrap();
  let updated = users::find_user_by_id(&mut *tx, &user.id).await.unwrap().unwrap();
  assert_eq!(updated.email, new_email);
  assert!(updated.email_verified_at.is_none(), "an email change resets the verified state");

  let invalidated = email_verifications::invalidate_email_verifications_for_user(&mut *tx, &user.id).await.unwrap();
  assert_eq!(invalidated, 1, "outstanding verification tokens are invalidated");
}

/* ── Email verification: send + validate ── */

#[tokio::test]
async fn send_verification_email_flow_at_the_query_level() {
  let pool = test_pool().await;
  let mut tx = pool.begin().await.unwrap();
  let user = users::create_user(&mut *tx, new_user(&unique_email())).await.unwrap();
  assert!(user.email_verified_at.is_none());

  // "Send": the verification row + the email_sends log row.
  let verification = email_verifications::create_email_verification(
    &mut *tx,
    email_verifications::NewEmailVerification {
      user_id: &user.id,
      email: &user.email,
      token_hash: "beef",
      expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
    },
  )
  .await
  .unwrap();
  let send = email_sends::create_email_send(
    &mut *tx,
    email_sends::NewEmailSend {
      user_id: Some(&user.id),
      to_email: &user.email,
      template: EmailTemplate::EmailVerify,
      status: EmailStatus::Queued,
    },
  )
  .await
  .unwrap();
  assert_eq!(send.template, EmailTemplate::EmailVerify);
  assert_eq!(send.status, EmailStatus::Queued);

  // The click: resolve by token hash, claim once, flip the user, audit it.
  let found = email_verifications::find_email_verification_by_token_hash(&mut *tx, "beef").await.unwrap().unwrap();
  assert_eq!(found.id, verification.id);
  assert_eq!(found.email, user.email, "the email snapshot rides the row");

  assert!(email_verifications::mark_email_verification_validated(&mut *tx, &verification.id).await.unwrap());
  assert!(
    !email_verifications::mark_email_verification_validated(&mut *tx, &verification.id).await.unwrap(),
    "a verification token is single-use"
  );

  users::mark_email_verified(&mut *tx, &user.id).await.unwrap();
  let verified = users::find_user_by_id(&mut *tx, &user.id).await.unwrap().unwrap();
  assert!(verified.email_verified_at.is_some());

  let audit = user_audit_logs::create_user_audit_log(
    &mut *tx,
    user_audit_logs::NewUserAuditLog {
      user_id: &user.id,
      actor_user_id: Some(&user.id),
      event: AuditEvent::EmailVerified,
      target_type: None,
      target_id: None,
      data: None,
      ip: None,
    },
  )
  .await
  .unwrap();
  assert!(audit.is_org_visible, "email_verified is org-visible (audit_logs.md §3)");
}
