//! Organization handlers (accounts.md §4, auth.md §4): the caller's account list (org
//! switcher), the members roster, and member removal. Invitation create/accept live in
//! `invitation_handlers`.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};

use db_queries::members;
use ids::{AccountId, UserId};

use crate::auth::service;
use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::handlers::auth_handlers::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// `GET /v1/accounts` — every organization the caller belongs to, with their role. Drives the
/// org switcher; the first entry is the oldest membership (the signup-created account).
pub async fn list_my_accounts(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<Value>> {
  let memberships = members::list_memberships_with_accounts(&state.pool, &auth.user.id).await?;
  let items: Vec<Value> = memberships
    .into_iter()
    .map(|m| {
      json!({
        "account_id": m.account_id,
        "name": m.account_name,
        "role": m.role,
        "joined_at": m.created_at,
      })
    })
    .collect();
  Ok(Json(json!({ "accounts": items })))
}

/// `GET /v1/accounts/{account_id}/members` — the roster. Any member may view it; outsiders get
/// the same 404 an unknown account would (no existence leak).
pub async fn list_account_members(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
) -> ApiResult<Json<Value>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  let my_membership = members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;

  let roster = members::list_members_of_account(&state.pool, &account_id).await?;
  let items: Vec<Value> = roster
    .into_iter()
    .map(|m| {
      json!({
        "user_id": m.user_id,
        "display_name": m.display_name,
        "email": m.email,
        "role": m.role,
        "joined_at": m.created_at,
        "is_you": m.user_id == auth.user.id,
      })
    })
    .collect();
  Ok(Json(json!({ "members": items, "your_role": my_membership.role })))
}

/// `DELETE /v1/accounts/{account_id}/members/{user_id}` — remove a member. Owner/admin only;
/// the service enforces the finer rules (no self-removal, owner immovable, admins can't remove
/// admins) and writes the `member_removed` audit rows.
pub async fn remove_account_member(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path((account_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  let target_user_id: UserId = user_id.parse().map_err(|_| ApiError::BadRequest("invalid user id".into()))?;

  let my_membership = require_account_role(&state.pool, &auth.user.id, &account_id, &["owner", "admin"]).await?;
  let meta = req_meta(&headers);
  service::remove_member(
    &state.pool,
    &auth.user,
    &my_membership.role,
    &account_id,
    &target_user_id,
    meta.ip.as_deref(),
  )
  .await?;
  Ok(Json(json!({ "ok": true })))
}
