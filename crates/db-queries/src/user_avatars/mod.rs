//! `user_avatars` table (accounts.md §2) — one row per user (UNIQUE on `user_id`). Holds the
//! avatar resolution state: display `source` (custom / gravatar / letter / generated), the
//! `gravatar_hash` (`sha256(trim+lower(email))`), and `gravatar_checked_at` — the last existence
//! probe. **`gravatar_checked_at IS NULL` is the "not yet polled" signal** the gravatar-job scans
//! for; it is set NULL on signup and reset to NULL on every email change.

use chrono::{DateTime, Utc};
use ids::{UserAvatarId, UserAvatarFileId, UserId};

mod create_user_avatar;
mod find_user_avatar_by_user;
mod list_unpolled_avatars;
mod mark_gravatar_checked;
mod remove_custom_avatar;
mod request_gravatar_repoll;
mod reset_avatar_for_email_change;
mod set_custom_avatar;
mod set_gravatar_dismissed;
mod set_gravatar_mirrored;

pub use create_user_avatar::{create_user_avatar, NewUserAvatar};
pub use find_user_avatar_by_user::find_user_avatar_by_user;
pub use list_unpolled_avatars::list_unpolled_avatars;
pub use mark_gravatar_checked::mark_gravatar_checked;
pub use remove_custom_avatar::remove_custom_avatar;
pub use request_gravatar_repoll::request_gravatar_repoll;
pub use reset_avatar_for_email_change::reset_avatar_for_email_change;
pub use set_custom_avatar::set_custom_avatar;
pub use set_gravatar_dismissed::set_gravatar_dismissed;
pub use set_gravatar_mirrored::set_gravatar_mirrored;

#[derive(Debug, Clone)]
pub struct UserAvatar {
  pub id: UserAvatarId,
  pub user_id: UserId,
  pub custom_file_id: Option<UserAvatarFileId>,
  pub gravatar_file_id: Option<UserAvatarFileId>,
  pub source: String,
  pub gravatar_hash: Option<String>,
  pub gravatar_checked_at: Option<DateTime<Utc>>,
  pub is_gravatar_dismissed: bool,
  pub generated_seed: String,
  pub email_first_letter: Option<String>,
  pub email_last_letter: Option<String>,
  pub person_name_first_letter: Option<String>,
  pub person_name_last_letter: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}
