//! `email_changes` table (accounts.md §2) — the append-only "email change log". One row per email
//! change, snapshotting all three tiers (canonical / clean / original) of BOTH the old and new
//! address. `change_source` is `user` (self-service) or `system` (e.g. soft-delete mangling).

use chrono::{DateTime, Utc};
use ids::{EmailChangeId, UserId};

mod create_email_change;

pub use create_email_change::{create_email_change, NewEmailChange};

#[derive(Debug, Clone)]
pub struct EmailChange {
  pub id: EmailChangeId,
  pub user_id: UserId,
  pub old_email: String,
  pub old_email_clean: String,
  pub old_email_original: String,
  pub new_email: String,
  pub new_email_clean: String,
  pub new_email_original: String,
  pub change_source: String,
  pub created_at: DateTime<Utc>,
}
