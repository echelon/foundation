use sqlx::PgExecutor;

use super::ApiKey;

/// Resolve a presented secret (by its sha256 hex) to a LIVE key — the Bearer front door
/// (auth.md §5). Live = not revoked and not expired. Stamps `last_used_at` in the same
/// statement so every successful authentication updates the dashboard's "last used".
pub async fn find_live_api_key_by_hash<'e, E: PgExecutor<'e>>(
  exec: E,
  key_hash: &str,
) -> sqlx::Result<Option<ApiKey>> {
  sqlx::query_as!(
    ApiKey,
    r#"UPDATE api_keys SET last_used_at = now()
       WHERE key_hash = $1 AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())
       RETURNING id AS "id: ids::ApiKeyId", account_id AS "account_id: ids::AccountId",
                 created_by_user_id AS "created_by_user_id: ids::UserId",
                 title, description, key_prefix,
                 created_at, last_used_at, expires_at, revoked_at"#,
    key_hash,
  )
  .fetch_optional(exec)
  .await
}
