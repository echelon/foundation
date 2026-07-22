use ids::{AccountId, FileId};
use sqlx::PgExecutor;

use super::FileRow;

pub async fn find_file_for_account<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  id: &FileId,
) -> sqlx::Result<Option<FileRow>> {
  sqlx::query_as!(
    FileRow,
    r#"SELECT id AS "id: ids::FileId", account_id AS "account_id: ids::AccountId", purpose, origin,
                 bucket_folder, bucket_hash, bucket_prefix, bucket_suffix, bucket_version,
                 bucket_extension, mime, bytes, created_at
       FROM files WHERE id = $1 AND account_id = $2"#,
    id.as_str(),
    account_id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
