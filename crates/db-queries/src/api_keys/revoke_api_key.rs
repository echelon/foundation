use ids::{AccountId, ApiKeyId};
use sqlx::PgExecutor;

/// Revoke ("delete") a key — scoped by account so a key id from another account can never be
/// touched. Returns rows affected (0 = unknown/foreign/already-revoked).
pub async fn revoke_api_key<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  id: &ApiKeyId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE api_keys SET revoked_at = now()
       WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL"#,
    id.as_str(),
    account_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
