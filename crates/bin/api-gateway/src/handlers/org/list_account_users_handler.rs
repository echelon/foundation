//! `GET /v1/accounts/{account_id}/users?ids=user_a,user_b` — the user-info API for rendering
//! people anywhere (audit-log actors, pickers, …): display name, email, and their chosen
//! avatar. Any member may call it; it only ever describes CURRENT members of that account,
//! so it can't be used to probe arbitrary user ids — unknown, foreign, or departed users are
//! silently omitted and clients fall back to rendering the bare id.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use db_queries::members;
use ids::{AccountId, UserId};

use crate::avatar_links::{avatar_infos, letter_only, AvatarInfo};
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

const MAX_IDS: usize = 100;

#[derive(Debug, Deserialize, IntoParams)]
pub struct UserInfoParams {
  /// Comma-separated user ids (`user_…`), max 100.
  pub ids: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountUserInfo {
  #[schema(value_type = String, example = "user_01j9…")]
  pub user_id: UserId,
  pub display_name: Option<String>,
  pub email: String,
  /// Their chosen avatar (custom upload, gravatar, or monogram).
  pub avatar: AvatarInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountUsersResponse {
  pub users: Vec<AccountUserInfo>,
}

#[utoipa::path(
  get, path = "/v1/accounts/{account_id}/users",
  tag = "org",
  security(("session_cookie" = [])),
  params(("account_id" = String, Path, description = "Account id (`acct_…`)"), UserInfoParams),
  responses(
    (status = 200, description = "Display info for the requested ids that are current members", body = AccountUsersResponse),
    (status = 400, description = "Invalid account id, or more than 100 ids"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn list_account_users_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
  Query(params): Query<UserInfoParams>,
) -> ApiResult<Json<AccountUsersResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;

  // Malformed ids are just "not a member" — skipped, not an error.
  let requested: Vec<UserId> = params
    .ids
    .split(',')
    .filter_map(|s| s.trim().parse::<UserId>().ok())
    .collect();
  if requested.len() > MAX_IDS {
    return Err(ApiError::BadRequest(format!("at most {MAX_IDS} ids per request")));
  }

  // The roster bounds visibility: only current members are ever described.
  let roster = members::list_members_of_account(&state.pool, &account_id).await?;
  let wanted: Vec<_> = roster.into_iter().filter(|m| requested.contains(&m.user_id)).collect();

  let ids: Vec<UserId> = wanted.iter().map(|m| m.user_id.clone()).collect();
  let mut avatars = avatar_infos(&state, &ids).await?;
  let users = wanted
    .into_iter()
    .map(|m| AccountUserInfo {
      avatar: avatars.remove(&m.user_id).unwrap_or_else(letter_only),
      user_id: m.user_id,
      display_name: m.display_name,
      email: m.email,
    })
    .collect();
  Ok(Json(AccountUsersResponse { users }))
}
