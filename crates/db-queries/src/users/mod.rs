//! `users` table (accounts.md §2).

use chrono::{DateTime, Utc};
use ids::UserId;

mod bump_session_version;
mod create_user;
mod find_user_by_email;
mod find_user_by_id;
mod set_password_hash;

pub use bump_session_version::bump_session_version;
pub use create_user::{create_user, NewUser};
pub use find_user_by_email::find_user_by_email;
pub use find_user_by_id::find_user_by_id;
pub use set_password_hash::set_password_hash;

/// A row from `users`. Field order MUST match the query column order.
#[derive(Debug, Clone)]
pub struct User {
  pub id: UserId,
  pub email: String,
  pub email_clean: String,
  pub email_original: String,
  pub email_verified_at: Option<DateTime<Utc>>,
  pub password_hash: Option<String>,
  pub session_version: i32,
  pub display_name: Option<String>,
  pub is_staff: bool,
  pub created_at: DateTime<Utc>,
}
