use ids::AccountId;
use sqlx::PgExecutor;

use super::ApiKey;

/// The account's un-revoked keys, newest first (the dashboard list).
pub async fn list_live_api_keys_for_account<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
) -> sqlx::Result<Vec<ApiKey>> {
  sqlx::query_as!(
    ApiKey,
    r#"SELECT id AS "id: ids::ApiKeyId", account_id AS "account_id: AccountId",
              created_by_user_id AS "created_by_user_id: ids::UserId",
              title, description, key_prefix,
              created_at, last_used_at, expires_at, revoked_at
       FROM api_keys
       WHERE account_id = $1 AND revoked_at IS NULL
       ORDER BY sid DESC"#,
    account_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
