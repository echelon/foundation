//! `account_members` table (accounts.md §2). Exactly one `owner` per account (partial unique
//! index); roles gate authorization (auth.md §4).

use chrono::{DateTime, Utc};
use ids::{AccountId, AccountMemberId, UserId};

mod create_membership;
mod delete_membership;
mod find_membership;
mod list_members_of_account;
mod list_memberships_for_user;
mod list_memberships_with_accounts;

pub use create_membership::create_membership;
pub use delete_membership::delete_membership;
pub use find_membership::find_membership;
pub use list_members_of_account::{list_members_of_account, AccountMemberDetail};
pub use list_memberships_for_user::list_memberships_for_user;
pub use list_memberships_with_accounts::{list_memberships_with_accounts, MembershipWithAccount};

#[derive(Debug, Clone)]
pub struct AccountMember {
  pub id: AccountMemberId,
  pub account_id: AccountId,
  pub user_id: UserId,
  pub role: enums::MemberRole,
  pub created_at: DateTime<Utc>,
}
