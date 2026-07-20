//! `user_avatar_files` table (accounts.md §2) — avatar image bytes live in R2; rows here hold
//! the bucket-path *pieces* (files.md §2 — key/URL derived in code, never stored). Rows are
//! immutable: crops become NEW rows linked via `original_user_avatar_file_id`.

use chrono::{DateTime, Utc};
use ids::{UserAvatarFileId, UserId};

mod create_user_avatar_file;
mod find_user_avatar_file_by_id;

pub use create_user_avatar_file::{create_user_avatar_file, NewUserAvatarFile};
pub use find_user_avatar_file_by_id::find_user_avatar_file_by_id;

#[derive(Debug, Clone)]
pub struct UserAvatarFile {
  pub id: UserAvatarFileId,
  pub user_id: UserId,
  /// Crop lineage: the uncropped source row, when this row is a baked crop.
  pub original_user_avatar_file_id: Option<UserAvatarFileId>,
  /// `custom_upload` / `gravatar_mirror`.
  pub avatar_source: String,
  pub bucket_hash: String,
  pub content_hash: String,
  pub bucket_suffix: Option<String>,
  pub bucket_extension: String,
  pub width: Option<i32>,
  pub height: Option<i32>,
  pub bytes: i64,
  pub created_at: DateTime<Utc>,
}
