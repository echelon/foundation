//! `account_members` table (accounts.md §2). Exactly one `owner` per account (partial unique
//! index); roles gate authorization (auth.md §4).

use chrono::{DateTime, Utc};
use ids::{AccountId, AccountMemberId, UserId};

mod create_membership;
mod find_membership;
mod list_memberships_for_user;

pub use create_membership::create_membership;
pub use find_membership::find_membership;
pub use list_memberships_for_user::list_memberships_for_user;

#[derive(Debug, Clone)]
pub struct AccountMember {
  pub id: AccountMemberId,
  pub account_id: AccountId,
  pub user_id: UserId,
  pub role: String,
  pub created_at: DateTime<Utc>,
}
