//! `sessions` table (accounts.md §2 / auth.md §1). The cookie is a signed JWT; the row is the
//! revocation authority (looked up by `id` every request).

use chrono::{DateTime, Utc};
use enums::SessionCreationType;
use ids::{SessionId, UserId};

mod create_session;
mod find_session_by_id;
mod list_live_sessions_for_user;
mod revoke_other_sessions;
mod revoke_session;

pub use create_session::{create_session, NewSession};
pub use find_session_by_id::find_session_by_id;
pub use list_live_sessions_for_user::list_live_sessions_for_user;
pub use revoke_other_sessions::revoke_other_sessions;
pub use revoke_session::revoke_session;

#[derive(Debug, Clone)]
pub struct Session {
  pub id: SessionId,
  pub user_id: UserId,
  pub impersonator_user_id: Option<UserId>,
  /// Creation IP, rendered as the bare address (`host(inet)` — no `/32` netmask suffix).
  /// Shown in the session/devices list.
  pub ip: Option<String>,
  pub user_agent: Option<String>,
  pub device: Option<String>,
  /// How the session came to exist (signup, invited_signup, login, sso_login, …).
  pub created_via: SessionCreationType,
  pub created_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
  pub revoked_at: Option<DateTime<Utc>>,
}
