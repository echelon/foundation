//! `user_sso_connections` table (accounts.md §5). Identity is resolved on
//! `(provider, provider_user_id)` — NEVER on email — to prevent account takeover.

use chrono::{DateTime, Utc};
use ids::{SsoConnectionId, UserId};

mod create_sso_connection;
mod delete_sso_connection;
mod find_sso_by_provider_sub;
mod list_sso_for_user;
mod touch_sso_last_used;

pub use create_sso_connection::{create_sso_connection, NewSsoConnection};
pub use delete_sso_connection::delete_sso_connection;
pub use find_sso_by_provider_sub::find_sso_by_provider_sub;
pub use list_sso_for_user::list_sso_for_user;
pub use touch_sso_last_used::touch_sso_last_used;

#[derive(Debug, Clone)]
pub struct SsoConnection {
  pub id: SsoConnectionId,
  pub user_id: UserId,
  pub provider: String,
  pub provider_user_id: String,
  pub provider_email: String,
  pub provider_user_display_name: Option<String>,
  pub created_at: DateTime<Utc>,
  pub last_used_at: DateTime<Utc>,
}
