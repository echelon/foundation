//! `accounts` table (accounts.md §2).

use chrono::{DateTime, Utc};
use ids::AccountId;

mod create_account;
mod find_account_by_id;
mod set_business_name;

pub use create_account::{create_account, NewAccount};
pub use find_account_by_id::find_account_by_id;
pub use set_business_name::set_business_name;

/// A row from `accounts` (exposed columns only — never `sid`). Field order MUST match the
/// `RETURNING`/`SELECT` column order in the queries (`query_as!` maps by position).
#[derive(Debug, Clone)]
pub struct Account {
  pub id: AccountId,
  pub name: Option<String>,
  /// Owner/admin-set display name for the business; null → clients show the placeholder.
  pub business_name: Option<String>,
  /// Random "Adjective Noun" minted at creation; never null.
  pub placeholder_business_name: String,
  pub status: String,
  pub created_at: DateTime<Utc>,
}
