//! `POST /v1/accounts/{account_id}/api_keys` — mint a key. The secret appears in THIS
//! response only; afterwards the API knows nothing but its hash.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use db_queries::members;
use ids::AccountId;

use super::types::ApiKeyItem;
use crate::auth::service;
use crate::error::{ApiError, ApiResult};
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

const MAX_TITLE_LEN: usize = 100;
const MAX_DESCRIPTION_LEN: usize = 500;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyBody {
  #[schema(example = "Production backend")]
  pub title: String,
  #[schema(example = "Used by the render farm workers")]
  pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
  pub api_key: ApiKeyItem,
  /// The full secret (`foundation_api_…`) — shown ONCE, never retrievable again.
  #[schema(example = "foundation_api_a1b2c3…")]
  pub secret: String,
}

/// Any member may create keys (auth.md §4, revised 2026-07-21).
#[utoipa::path(
  post, path = "/v1/accounts/{account_id}/api_keys",
  tag = "api-keys",
  security(("session_cookie" = [])),
  params(("account_id" = String, Path, description = "Account id (`acct_…`)")),
  request_body = CreateApiKeyBody,
  responses(
    (status = 200, description = "Key minted; `secret` is shown once", body = CreateApiKeyResponse),
    (status = 400, description = "Invalid account id, blank title, or over-long title/description"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn create_api_key_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path(account_id): Path<String>,
  Json(body): Json<CreateApiKeyBody>,
) -> ApiResult<Json<CreateApiKeyResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;

  let title = body.title.trim();
  if title.is_empty() {
    return Err(ApiError::BadRequest("give the key a name".into()));
  }
  if title.chars().count() > MAX_TITLE_LEN {
    return Err(ApiError::BadRequest(format!("key name is limited to {MAX_TITLE_LEN} characters")));
  }
  let description = body.description.as_deref().map(str::trim).filter(|s| !s.is_empty());
  if description.is_some_and(|d| d.chars().count() > MAX_DESCRIPTION_LEN) {
    return Err(ApiError::BadRequest(format!("description is limited to {MAX_DESCRIPTION_LEN} characters")));
  }

  let meta = req_meta(&headers);
  let created = service::create_api_key(
    &state.pool,
    &account_id,
    &auth.user,
    service::CreateApiKeyArgs { title, description, ip: meta.ip.as_deref() },
  )
  .await?;
  Ok(Json(CreateApiKeyResponse { api_key: created.api_key.into(), secret: created.secret }))
}
