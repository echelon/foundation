//! `email_verifications` table (accounts.md §2). A verification emails a plaintext token; only its
//! sha256 `token_hash` is stored. The `email` snapshot means a token can never verify an address it
//! wasn't minted for. Dormant at launch (no hard verification requirement), but fully wired.

use chrono::{DateTime, Utc};
use ids::{EmailVerificationId, UserId};

mod create_email_verification;
mod find_email_verification_by_token_hash;
mod invalidate_email_verifications_for_user;
mod mark_email_verification_validated;

pub use create_email_verification::{create_email_verification, NewEmailVerification};
pub use find_email_verification_by_token_hash::find_email_verification_by_token_hash;
pub use invalidate_email_verifications_for_user::invalidate_email_verifications_for_user;
pub use mark_email_verification_validated::mark_email_verification_validated;

#[derive(Debug, Clone)]
pub struct EmailVerification {
  pub id: EmailVerificationId,
  pub user_id: UserId,
  /// Canonicalized address snapshot at send time — validation requires it still matches the user.
  pub email: String,
  pub token_hash: String,
  pub created_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
  pub validated_at: Option<DateTime<Utc>>,
}
