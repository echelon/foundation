//! The API key as lists report it — prefix only, never a secret or hash.

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use db_queries::ApiKey;
use ids::{ApiKeyId, UserId};

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyItem {
  #[schema(value_type = String, example = "key_01j9…")]
  pub id: ApiKeyId,
  pub title: String,
  pub description: Option<String>,
  /// Identifying prefix of the secret (`foundation_api_xxxxxx`) — the full secret is shown
  /// only once, at creation.
  #[schema(example = "foundation_api_a1b2c3")]
  pub key_prefix: String,
  /// Who minted it — resolve display info via `/v1/accounts/{id}/users`.
  #[schema(value_type = String, example = "user_01j9…")]
  pub created_by_user_id: UserId,
  pub created_at: DateTime<Utc>,
  pub last_used_at: Option<DateTime<Utc>>,
  pub expires_at: Option<DateTime<Utc>>,
}

impl From<ApiKey> for ApiKeyItem {
  fn from(k: ApiKey) -> Self {
    Self {
      id: k.id,
      title: k.title,
      description: k.description,
      key_prefix: k.key_prefix,
      created_by_user_id: k.created_by_user_id,
      created_at: k.created_at,
      last_used_at: k.last_used_at,
      expires_at: k.expires_at,
    }
  }
}
