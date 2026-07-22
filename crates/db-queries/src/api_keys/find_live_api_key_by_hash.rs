use sqlx::PgExecutor;

use super::ApiKey;

/// Resolve a presented secret (by its sha256 hex) to a LIVE key — the Bearer front door
/// (auth.md §5). Live = not revoked and not expired. READ-ONLY: the hot auth path never
/// writes; `last_used_at` freshness is [`touch_api_key_last_used`] (coarse, inline) plus
/// the background job's jobs-derived refresh.
pub async fn find_live_api_key_by_hash<'e, E: PgExecutor<'e>>(
  exec: E,
  key_hash: &str,
) -> sqlx::Result<Option<ApiKey>> {
  sqlx::query_as!(
    ApiKey,
    r#"SELECT id AS "id: ids::ApiKeyId", account_id AS "account_id: ids::AccountId",
              created_by_user_id AS "created_by_user_id: ids::UserId",
              title, description, key_prefix,
              created_at, last_used_at, expires_at, revoked_at
       FROM api_keys
       WHERE key_hash = $1 AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())"#,
    key_hash,
  )
  .fetch_optional(exec)
  .await
}
