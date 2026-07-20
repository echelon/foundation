//! `email_sends` table (email.md §3) — the send log. One row per outbound email: which template,
//! to whom, and its lifecycle `status` (`queued → sent → delivered | bounced | complained |
//! failed`). `provider_message_id` links to Resend for delivery-event reconciliation.

use chrono::{DateTime, Utc};
use ids::{EmailSendId, UserId};

mod create_email_send;
mod update_email_send_status;

pub use create_email_send::{create_email_send, NewEmailSend};
pub use update_email_send_status::update_email_send_status;

#[derive(Debug, Clone)]
pub struct EmailSend {
  pub id: EmailSendId,
  pub user_id: Option<UserId>,
  pub to_email: String,
  pub template: String,
  pub status: String,
  pub provider_message_id: Option<String>,
  pub error: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}
