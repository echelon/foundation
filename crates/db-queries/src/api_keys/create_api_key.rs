use ids::{AccountId, ApiKeyId, UserId};
use sqlx::PgExecutor;

use super::ApiKey;

#[derive(Debug, Clone)]
pub struct NewApiKey<'a> {
  pub account_id: &'a AccountId,
  pub created_by_user_id: &'a UserId,
  pub title: &'a str,
  pub description: Option<&'a str>,
  /// Display prefix of the secret (identifying, not authenticating).
  pub key_prefix: &'a str,
  /// sha256 hex of the full secret — the only credential-shaped thing persisted.
  pub key_hash: &'a str,
}

pub async fn create_api_key<'e, E: PgExecutor<'e>>(exec: E, new: NewApiKey<'_>) -> sqlx::Result<ApiKey> {
  let id = ApiKeyId::generate();
  sqlx::query_as!(
    ApiKey,
    r#"INSERT INTO api_keys (id, account_id, created_by_user_id, title, description, key_prefix, key_hash)
       VALUES ($1, $2, $3, $4, $5, $6, $7)
       RETURNING id AS "id: ApiKeyId", account_id AS "account_id: AccountId",
                 created_by_user_id AS "created_by_user_id: UserId",
                 title, description, key_prefix,
                 created_at, last_used_at, expires_at, revoked_at"#,
    id.as_str(),
    new.account_id.as_str(),
    new.created_by_user_id.as_str(),
    new.title,
    new.description,
    new.key_prefix,
    new.key_hash,
  )
  .fetch_one(exec)
  .await
}
