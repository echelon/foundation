//! `GET /v1/accounts/{account_id}/api_keys` — the account's live keys.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use db_queries::{api_keys, members};
use ids::AccountId;

use super::types::ApiKeyItem;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeysResponse {
  pub api_keys: Vec<ApiKeyItem>,
}

/// Any member may view (non-members get the same 404 an unknown account would).
#[utoipa::path(
  get, path = "/v1/accounts/{account_id}/api_keys",
  tag = "api-keys",
  security(("session_cookie" = [])),
  params(("account_id" = String, Path, description = "Account id (`acct_…`)")),
  responses(
    (status = 200, description = "Un-revoked keys, newest first", body = ApiKeysResponse),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn list_api_keys_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
) -> ApiResult<Json<ApiKeysResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;

  let keys = api_keys::list_live_api_keys_for_account(&state.pool, &account_id).await?;
  Ok(Json(ApiKeysResponse { api_keys: keys.into_iter().map(Into::into).collect() }))
}
