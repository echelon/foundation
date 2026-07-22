//! `api_keys` table (auth.md §4, schema §identity). The plaintext secret is NEVER stored —
//! only its sha256 (`key_hash`, deliberately absent from the row struct so it can't leak
//! through an API response) plus a short display prefix.

use chrono::{DateTime, Utc};
use ids::{AccountId, ApiKeyId, UserId};

mod create_api_key;
mod find_live_api_key_by_hash;
mod list_live_api_keys_for_account;
mod refresh_api_keys_last_used_from_jobs;
mod revoke_api_key;
mod touch_api_key_last_used;

pub use create_api_key::{create_api_key, NewApiKey};
pub use find_live_api_key_by_hash::find_live_api_key_by_hash;
pub use list_live_api_keys_for_account::list_live_api_keys_for_account;
pub use refresh_api_keys_last_used_from_jobs::refresh_api_keys_last_used_from_jobs;
pub use revoke_api_key::revoke_api_key;
pub use touch_api_key_last_used::touch_api_key_last_used;

/// A row from `api_keys` (exposed columns only — never `sid`, never `key_hash`). Field order
/// MUST match the `RETURNING`/`SELECT` column order in the queries.
#[derive(Debug, Clone)]
pub struct ApiKey {
  pub id: ApiKeyId,
  pub account_id: AccountId,
  pub created_by_user_id: UserId,
  pub title: String,
  pub description: Option<String>,
  /// Display prefix of the secret (`foundation_api_xxxxx`) — enough to identify, useless to
  /// authenticate.
  pub key_prefix: String,
  pub created_at: DateTime<Utc>,
  pub last_used_at: Option<DateTime<Utc>>,
  pub expires_at: Option<DateTime<Utc>>,
  pub revoked_at: Option<DateTime<Utc>>,
}
