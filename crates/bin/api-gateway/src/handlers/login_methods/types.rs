//! The login-methods list shape, shared by GET and the link endpoint's response.

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use db_queries::{sso, User};
use ids::{SsoConnectionId, UserId};

use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordMethod {
  /// Password auth can never be removed once added (accounts.md §5), so there is no id here.
  pub is_enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SsoConnectionItem {
  #[schema(value_type = String, example = "sso_01j9…")]
  pub id: SsoConnectionId,
  #[schema(example = "google")]
  pub provider: String,
  pub provider_email: String,
  pub created_at: DateTime<Utc>,
  pub last_used_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginMethodsResponse {
  pub password: PasswordMethod,
  pub sso_connections: Vec<SsoConnectionItem>,
}

pub async fn login_methods_response(state: &AppState, user: &User) -> ApiResult<LoginMethodsResponse> {
  let user_id: &UserId = &user.id;
  let connections = sso::list_sso_for_user(&state.pool, user_id).await?;
  Ok(LoginMethodsResponse {
    password: PasswordMethod { is_enabled: user.password_hash.is_some() },
    sso_connections: connections
      .into_iter()
      .map(|c| SsoConnectionItem {
        id: c.id,
        provider: c.provider,
        provider_email: c.provider_email,
        created_at: c.created_at,
        last_used_at: c.last_used_at,
      })
      .collect(),
  })
}
