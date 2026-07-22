//! `files` table (files.md §2) — the storage primitive. CDN URLs are DERIVED from the
//! bucket-path pieces (never stored).

use chrono::{DateTime, Utc};
use ids::{AccountId, FileId};

mod create_file;
mod find_file_for_account;

pub use create_file::{create_file, NewFile};
pub use find_file_for_account::find_file_for_account;

#[derive(Debug, Clone)]
pub struct FileRow {
  pub id: FileId,
  pub account_id: AccountId,
  pub purpose: String,
  pub origin: String,
  pub bucket_folder: String,
  pub bucket_hash: String,
  pub bucket_prefix: Option<String>,
  pub bucket_suffix: Option<String>,
  pub bucket_version: Option<i32>,
  pub bucket_extension: Option<String>,
  pub mime: String,
  pub bytes: i64,
  pub created_at: DateTime<Utc>,
}
