//! `GET /v1/accounts` — every organization the caller belongs to, with their role.

use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use db_queries::members;
use ids::AccountId;

use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountItem {
  #[schema(value_type = String, example = "acct_01j9…")]
  pub account_id: AccountId,
  pub name: Option<String>,
  /// Owner/admin-set; null → display `placeholder_business_name`.
  pub business_name: Option<String>,
  #[schema(example = "Radiant Compass")]
  pub placeholder_business_name: String,
  #[schema(example = "owner")]
  pub role: String,
  pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountsResponse {
  pub accounts: Vec<AccountItem>,
}

/// Drives the org switcher; the first entry is the oldest membership (the signup-created
/// account).
#[utoipa::path(
  get, path = "/v1/accounts",
  tag = "org",
  security(("session_cookie" = [])),
  responses((status = 200, description = "The caller's organizations, oldest membership first", body = AccountsResponse))
)]
pub async fn list_my_accounts_handler(
  State(state): State<AppState>,
  auth: AuthContext,
) -> ApiResult<Json<AccountsResponse>> {
  let memberships = members::list_memberships_with_accounts(&state.pool, &auth.user.id).await?;
  let accounts = memberships
    .into_iter()
    .map(|m| AccountItem {
      account_id: m.account_id,
      name: m.account_name,
      business_name: m.business_name,
      placeholder_business_name: m.placeholder_business_name,
      role: m.role.to_string(),
      joined_at: m.created_at,
    })
    .collect();
  Ok(Json(AccountsResponse { accounts }))
}
