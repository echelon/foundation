//! `DELETE /v1/accounts/{account_id}/api_keys/{key_id}` — revoke ("delete") a key.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;

use db_queries::members;
use ids::{AccountId, ApiKeyId};

use crate::auth::service;
use crate::error::{ApiError, ApiResult};
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Any member may delete keys (auth.md §4, revised 2026-07-21). The row is kept (revoked)
/// for the audit trail; the key stops authenticating immediately.
#[utoipa::path(
  delete, path = "/v1/accounts/{account_id}/api_keys/{key_id}",
  tag = "api-keys",
  security(("session_cookie" = [])),
  params(
    ("account_id" = String, Path, description = "Account id (`acct_…`)"),
    ("key_id" = String, Path, description = "API key id (`key_…`)")
  ),
  responses(
    (status = 200, description = "Key revoked", body = OkResponse),
    (status = 400, description = "Invalid id"),
    (status = 404, description = "Unknown account/key, foreign key, or the caller is not a member")
  )
)]
pub async fn delete_api_key_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path((account_id, key_id)): Path<(String, String)>,
) -> ApiResult<Json<OkResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  let key_id: ApiKeyId = key_id.parse().map_err(|_| ApiError::BadRequest("invalid api key id".into()))?;
  members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;

  let meta = req_meta(&headers);
  service::revoke_api_key(&state.pool, &account_id, &auth.user, &key_id, meta.ip.as_deref()).await?;
  Ok(Json(OkResponse::ok()))
}
