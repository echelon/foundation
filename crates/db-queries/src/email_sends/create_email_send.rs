use enums::{EmailStatus, EmailTemplate};
use ids::{EmailSendId, UserId};
use sqlx::PgExecutor;

use super::EmailSend;

#[derive(Debug, Clone)]
pub struct NewEmailSend<'a> {
  /// NULL for pre-user sends (e.g. an invite to a not-yet-registered address).
  pub user_id: Option<&'a UserId>,
  pub to_email: &'a str,
  pub template: EmailTemplate,
  /// Initial status, typically `Queued`.
  pub status: EmailStatus,
}

pub async fn create_email_send<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewEmailSend<'_>,
) -> sqlx::Result<EmailSend> {
  let id = EmailSendId::generate();
  let user = new.user_id.map(|u| u.as_str());
  sqlx::query_as!(
    EmailSend,
    r#"INSERT INTO email_sends (id, user_id, to_email, template, status)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING id AS "id: EmailSendId", user_id AS "user_id?: UserId",
                 to_email, template AS "template: EmailTemplate", status AS "status: EmailStatus", provider_message_id, error, created_at, updated_at"#,
    id.as_str(),
    user,
    new.to_email,
    new.template.as_str(),
    new.status.as_str(),
  )
  .fetch_one(exec)
  .await
}
