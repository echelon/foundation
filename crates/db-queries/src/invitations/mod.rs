//! `invitations` table (accounts.md §4). An invite carries a short human `code` and a magic-link
//! token (only its sha256 `token_hash` is stored). Invites grant `admin`/`member`, never `owner`.

use chrono::{DateTime, Utc};
use ids::{AccountId, InvitationId, UserId};

mod create_invitation;
mod find_invitation_by_code;
mod find_invitation_by_id;
mod find_invitation_by_token_hash;
mod list_pending_invitations_for_account;
mod mark_invitation_accepted;
mod revoke_invitation;
mod revoke_outstanding_invitations;

pub use create_invitation::{create_invitation, NewInvitation};
pub use find_invitation_by_code::find_invitation_by_code;
pub use find_invitation_by_id::find_invitation_by_id;
pub use find_invitation_by_token_hash::find_invitation_by_token_hash;
pub use list_pending_invitations_for_account::list_pending_invitations_for_account;
pub use mark_invitation_accepted::mark_invitation_accepted;
pub use revoke_invitation::revoke_invitation;
pub use revoke_outstanding_invitations::revoke_outstanding_invitations;

#[derive(Debug, Clone)]
pub struct Invitation {
  pub id: InvitationId,
  pub account_id: AccountId,
  pub invited_by_user_id: UserId,
  pub accepted_by_user_id: Option<UserId>,
  pub email: String,
  pub role: enums::MemberRole,
  pub code: String,
  pub token_hash: String,
  pub created_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
  pub accepted_at: Option<DateTime<Utc>>,
  pub revoked_at: Option<DateTime<Utc>>,
}
