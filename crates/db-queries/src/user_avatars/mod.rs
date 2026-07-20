//! `user_avatars` table (accounts.md §2) — one row per user (UNIQUE on `user_id`).
//!
//! `preferred_source` is the USER'S CHOICE of what to display (`custom` / `gravatar` /
//! `letter` / `generated`) — REVISED 2026-07-20, replacing the old derived `source` +
//! `is_gravatar_dismissed` pair. The *effective* display is derived in code: the preference
//! when it's satisfiable (custom needs `custom_file_id`, gravatar needs `gravatar_file_id`),
//! else the letter monogram. Keeping a mirror around while preferring something else IS the
//! old "dismissed" state.
//!
//! `gravatar_checked_at IS NULL` is the "needs a (re-)poll" signal the gravatar-job scans for;
//! it's NULL on signup, reset on email change, and reset by an explicit refresh request.

use chrono::{DateTime, Utc};
use enums::AvatarSource;
use ids::{UserAvatarId, UserAvatarFileId, UserId};

mod create_user_avatar;
mod find_user_avatar_by_user;
mod list_unpolled_avatars;
mod mark_gravatar_checked;
mod remove_custom_avatar;
mod remove_gravatar;
mod request_gravatar_repoll;
mod reset_avatar_for_email_change;
mod set_custom_avatar;
mod set_gravatar_mirrored;
mod set_person_letters;
mod set_preferred_source;

pub use create_user_avatar::{create_user_avatar, NewUserAvatar};
pub use find_user_avatar_by_user::find_user_avatar_by_user;
pub use list_unpolled_avatars::list_unpolled_avatars;
pub use mark_gravatar_checked::mark_gravatar_checked;
pub use remove_custom_avatar::remove_custom_avatar;
pub use remove_gravatar::remove_gravatar;
pub use request_gravatar_repoll::request_gravatar_repoll;
pub use reset_avatar_for_email_change::reset_avatar_for_email_change;
pub use set_custom_avatar::set_custom_avatar;
pub use set_gravatar_mirrored::set_gravatar_mirrored;
pub use set_person_letters::set_person_letters;
pub use set_preferred_source::set_preferred_source;

#[derive(Debug, Clone)]
pub struct UserAvatar {
  pub id: UserAvatarId,
  pub user_id: UserId,
  pub custom_file_id: Option<UserAvatarFileId>,
  pub gravatar_file_id: Option<UserAvatarFileId>,
  /// The user's display choice — NOT necessarily what's shown (see module docs).
  pub preferred_source: AvatarSource,
  pub gravatar_hash: Option<String>,
  pub gravatar_checked_at: Option<DateTime<Utc>>,
  pub generated_seed: String,
  pub email_first_letter: Option<String>,
  pub email_last_letter: Option<String>,
  pub person_name_first_letter: Option<String>,
  pub person_name_last_letter: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}
