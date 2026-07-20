//! `GET /v1/accounts/{account_id}/members` — the roster.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use db_queries::members;
use ids::{AccountId, UserId};

use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberItem {
  #[schema(value_type = String, example = "user_01j9…")]
  pub user_id: UserId,
  pub display_name: Option<String>,
  pub email: String,
  #[schema(example = "member")]
  pub role: String,
  pub joined_at: DateTime<Utc>,
  /// Marks the caller's own row.
  pub is_you: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MembersResponse {
  pub members: Vec<MemberItem>,
  #[schema(example = "admin")]
  pub your_role: String,
}

/// Any member may view it; outsiders get the same 404 an unknown account would (no existence
/// leak).
#[utoipa::path(
  get, path = "/v1/accounts/{account_id}/members",
  tag = "org",
  security(("session_cookie" = [])),
  params(("account_id" = String, Path, description = "Account id (`acct_…`)")),
  responses(
    (status = 200, description = "The roster, oldest member (the owner) first", body = MembersResponse),
    (status = 400, description = "Invalid account id"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn list_account_members_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
) -> ApiResult<Json<MembersResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  let my_membership = members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;

  let roster = members::list_members_of_account(&state.pool, &account_id).await?;
  let members = roster
    .into_iter()
    .map(|m| MemberItem {
      is_you: m.user_id == auth.user.id,
      user_id: m.user_id,
      display_name: m.display_name,
      email: m.email,
      role: m.role,
      joined_at: m.created_at,
    })
    .collect();
  Ok(Json(MembersResponse { members, your_role: my_membership.role }))
}
