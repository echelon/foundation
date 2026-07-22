//! `PUT /v1/accounts/{account_id}/business` — set or clear the business name.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use ids::AccountId;

use crate::auth::service;
use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

const MAX_BUSINESS_NAME_LEN: usize = 120;

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateBusinessNameBody {
  /// The new name; `null` (or blank) clears it, falling back to the placeholder.
  #[schema(example = "Acme Corporation")]
  pub business_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BusinessNameResponse {
  pub ok: bool,
  /// Null when unset — clients display `placeholder_business_name` instead.
  pub business_name: Option<String>,
  pub placeholder_business_name: String,
}

/// Owner/admin only; members get 403, non-members the same 404 an unknown account would.
#[utoipa::path(
  put, path = "/v1/accounts/{account_id}/business",
  tag = "org",
  security(("session_cookie" = [])),
  params(("account_id" = String, Path, description = "Account id (`acct_…`)")),
  request_body = UpdateBusinessNameBody,
  responses(
    (status = 200, description = "Business name updated", body = BusinessNameResponse),
    (status = 400, description = "Invalid account id, or the name is too long"),
    (status = 403, description = "Caller is a member but not owner/admin"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn update_business_name_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path(account_id): Path<String>,
  Json(body): Json<UpdateBusinessNameBody>,
) -> ApiResult<Json<BusinessNameResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, enums::MemberRole::ADMINS).await?;

  // Blank clears; the placeholder always remains as the fallback display.
  let business_name = body.business_name.as_deref().map(str::trim).filter(|s| !s.is_empty());
  if business_name.is_some_and(|s| s.chars().count() > MAX_BUSINESS_NAME_LEN) {
    return Err(ApiError::BadRequest(format!("business name is limited to {MAX_BUSINESS_NAME_LEN} characters")));
  }

  let meta = req_meta(&headers);
  let account =
    service::update_business_name(&state.pool, &account_id, &auth.user.id, business_name, meta.ip.as_deref()).await?;
  Ok(Json(BusinessNameResponse {
    ok: true,
    business_name: account.business_name,
    placeholder_business_name: account.placeholder_business_name,
  }))
}
