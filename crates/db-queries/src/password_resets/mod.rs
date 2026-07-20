//! `password_resets` table (accounts.md §2). A reset emails a plaintext token; only its sha256
//! `token_hash` is stored. Short-lived (~1h) and single-use (`used_at`). Live at launch.

use chrono::{DateTime, Utc};
use ids::{PasswordResetId, UserId};

mod create_password_reset;
mod find_password_reset_by_token_hash;
mod mark_password_reset_used;

pub use create_password_reset::create_password_reset;
pub use find_password_reset_by_token_hash::find_password_reset_by_token_hash;
pub use mark_password_reset_used::mark_password_reset_used;

#[derive(Debug, Clone)]
pub struct PasswordReset {
  pub id: PasswordResetId,
  pub user_id: UserId,
  pub token_hash: String,
  pub created_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
  pub used_at: Option<DateTime<Utc>>,
}
