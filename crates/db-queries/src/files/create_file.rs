use ids::{AccountId, FileId};
use sqlx::PgExecutor;

use super::FileRow;

#[derive(Debug)]
pub struct NewFile<'a> {
  pub account_id: &'a AccountId,
  /// `input` (uploads) or `output` (generations).
  pub purpose: &'a str,
  /// `upload` / `mirrored_url` / `generated`.
  pub origin: &'a str,
  pub bucket_folder: &'a str,
  pub bucket_hash: &'a str,
  pub bucket_extension: Option<&'a str>,
  pub mime: &'a str,
  pub bytes: i64,
  pub checksum_sha256: &'a str,
}

pub async fn create_file<'e, E: PgExecutor<'e>>(exec: E, new: NewFile<'_>) -> sqlx::Result<FileRow> {
  let id = FileId::generate();
  sqlx::query_as!(
    FileRow,
    r#"INSERT INTO files
         (id, account_id, purpose, origin, bucket_folder, bucket_hash, bucket_extension,
          mime, bytes, checksum_sha256, storage_class)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'hot')
       RETURNING id AS "id: ids::FileId", account_id AS "account_id: ids::AccountId", purpose, origin,
                 bucket_folder, bucket_hash, bucket_prefix, bucket_suffix, bucket_version,
                 bucket_extension, mime, bytes, created_at"#,
    id.as_str(),
    new.account_id.as_str(),
    new.purpose,
    new.origin,
    new.bucket_folder,
    new.bucket_hash,
    new.bucket_extension,
    new.mime,
    new.bytes,
    new.checksum_sha256,
  )
  .fetch_one(exec)
  .await
}
