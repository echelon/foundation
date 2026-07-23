//! API keys: mint (secret shown once) and revoke (auth.md §4).


use db_queries::{
  account_audit_logs, User,
};
use enums::AuditEvent;
use ids::AccountId;

use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// A freshly-minted API key: the row plus the plaintext secret, which exists ONLY in this
/// return value (the DB stores its sha256) — show it once and let it go.
pub struct CreatedApiKey {
  pub api_key: db_queries::ApiKey,
  pub secret: String,
}

/// What `create_api_key` needs beyond ids (named fields per the args-struct rule).
pub struct CreateApiKeyArgs<'a> {
  pub title: &'a str,
  pub description: Option<&'a str>,
  pub ip: Option<&'a str>,
}

/// Mint an API key for the account (any member — auth.md §4, revised 2026-07-21). The secret
/// is the canonical `foundation_api_…` form (api_overview.md); the stored prefix identifies
/// the key in lists without being able to authenticate. Audited in the insert's transaction.
pub async fn create_api_key(
  pool: &db::PgPool,
  account_id: &AccountId,
  creator: &User,
  args: CreateApiKeyArgs<'_>,
) -> ApiResult<CreatedApiKey> {
  // 32 random bytes, hex — same entropy family as the magic-link tokens.
  let secret = {
    use rand::Rng;
    let bytes: [u8; 32] = rand::thread_rng().gen();
    format!("foundation_api_{}", hex::encode(bytes))
  };
  // "foundation_api_" + 6 identifying chars.
  let key_prefix = &secret[..("foundation_api_".len() + 6)];

  let mut tx = pool.begin().await?;
  let api_key = db_queries::api_keys::create_api_key(
    &mut *tx,
    db_queries::api_keys::NewApiKey {
      account_id,
      created_by_user_id: &creator.id,
      title: args.title,
      description: args.description,
      key_prefix,
      key_hash: &sha256_hex(&secret),
    },
  )
  .await?;
  account_audit_logs::create_account_audit_log(
    &mut *tx,
    account_audit_logs::NewAccountAuditLog {
      account_id,
      actor_user_id: Some(&creator.id),
      user_id: None,
      event: AuditEvent::ApiKeyCreated,
      target_type: Some("api_key"),
      target_id: Some(api_key.id.as_str()),
      data: Some(crate::audit_data::AuditData { title: Some(args.title.to_string()), key_prefix: Some(key_prefix.to_string()), ..Default::default() }.value()),
      ip: args.ip,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(CreatedApiKey { api_key, secret })
}

/// Revoke ("delete") an API key — any member (auth.md §4, revised 2026-07-21). 404 for an
/// unknown, foreign, or already-revoked key. Audited in the revoke's transaction.
pub async fn revoke_api_key(
  pool: &db::PgPool,
  account_id: &AccountId,
  actor: &User,
  api_key_id: &ids::ApiKeyId,
  ip: Option<&str>,
) -> ApiResult<()> {
  let mut tx = pool.begin().await?;
  let revoked = db_queries::api_keys::revoke_api_key(&mut *tx, account_id, api_key_id).await?;
  if revoked == 0 {
    return Err(ApiError::NotFound("api key not found".into()));
  }
  account_audit_logs::create_account_audit_log(
    &mut *tx,
    account_audit_logs::NewAccountAuditLog {
      account_id,
      actor_user_id: Some(&actor.id),
      user_id: None,
      event: AuditEvent::ApiKeyRevoked,
      target_type: Some("api_key"),
      target_id: Some(api_key_id.as_str()),
      data: None,
      ip,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
