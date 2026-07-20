//! `accounts` table (accounts.md §2).

use chrono::{DateTime, Utc};
use ids::AccountId;

mod create_account;
mod find_account_by_id;

pub use create_account::create_account;
pub use find_account_by_id::find_account_by_id;

/// A row from `accounts` (exposed columns only — never `sid`). Field order MUST match the
/// `RETURNING`/`SELECT` column order in the queries (`query_as!` maps by position).
#[derive(Debug, Clone)]
pub struct Account {
  pub id: AccountId,
  pub name: Option<String>,
  pub status: String,
  pub created_at: DateTime<Utc>,
}
