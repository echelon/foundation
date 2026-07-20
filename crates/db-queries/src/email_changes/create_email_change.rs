use ids::{EmailChangeId, UserId};
use sqlx::PgExecutor;

use super::EmailChange;

#[derive(Debug, Clone)]
pub struct NewEmailChange<'a> {
  pub user_id: &'a UserId,
  pub old_email: &'a str,
  pub old_email_clean: &'a str,
  pub old_email_original: &'a str,
  pub new_email: &'a str,
  pub new_email_clean: &'a str,
  pub new_email_original: &'a str,
  /// CHECK-constrained to `user` / `system`.
  pub change_source: &'a str,
}

pub async fn create_email_change<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewEmailChange<'_>,
) -> sqlx::Result<EmailChange> {
  let id = EmailChangeId::generate();
  sqlx::query_as!(
    EmailChange,
    r#"INSERT INTO email_changes
         (id, user_id, old_email, old_email_clean, old_email_original,
          new_email, new_email_clean, new_email_original, change_source)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
       RETURNING id AS "id: EmailChangeId", user_id AS "user_id: UserId",
                 old_email, old_email_clean, old_email_original,
                 new_email, new_email_clean, new_email_original,
                 change_source, created_at"#,
    id.as_str(),
    new.user_id.as_str(),
    new.old_email,
    new.old_email_clean,
    new.old_email_original,
    new.new_email,
    new.new_email_clean,
    new.new_email_original,
    new.change_source,
  )
  .fetch_one(exec)
  .await
}
