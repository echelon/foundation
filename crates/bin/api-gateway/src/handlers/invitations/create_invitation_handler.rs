//! `POST /v1/accounts/{account_id}/invitations` — invite someone into the org. Owner/admin only.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use ids::{AccountId, InvitationId};

use crate::auth::service;
use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::mailer;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateInvitationBody {
  pub email: String,
  /// `admin` | `member` (invites never grant `owner`).
  #[schema(example = "member")]
  pub role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateInvitationResponse {
  #[schema(value_type = String, example = "inv_01j9…")]
  pub invitation_id: InvitationId,
  /// The short human code (e.g. `7F3K-QT9M`).
  pub code: String,
  /// The magic link carries the plaintext token (stored only as a hash).
  pub join_url: String,
  pub email: String,
  pub role: String,
  pub expires_at: DateTime<Utc>,
}

#[utoipa::path(
  post, path = "/v1/accounts/{account_id}/invitations",
  tag = "invitations",
  security(("session_cookie" = [])),
  params(("account_id" = String, Path, description = "Account id (`acct_…`)")),
  request_body = CreateInvitationBody,
  responses(
    (status = 200, description = "Invitation created (any outstanding invite for this email was revoked)", body = CreateInvitationResponse),
    (status = 400, description = "Invalid account id, email, or role"),
    (status = 404, description = "Not a member, or not admin/owner"),
    (status = 409, description = "That email is already a member of this account")
  )
)]
pub async fn create_invitation_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
  Json(body): Json<CreateInvitationBody>,
) -> ApiResult<Json<CreateInvitationResponse>> {
  let account_id: AccountId = account_id
    .parse()
    .map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, &["owner", "admin"]).await?;

  let created = service::create_invitation(&state.pool, &account_id, &auth.user.id, &body.email, &body.role).await?;

  // The invite email (magic link + code) — best-effort: a mail failure never fails the
  // request; the console still shows the code/link, and `email_sends` records the outcome.
  let account_name = db_queries::accounts::find_account_by_id(&state.pool, &account_id)
    .await?
    .and_then(|a| a.name);
  mailer::send_invitation_email(
    &state,
    mailer::InvitationEmailArgs {
      to_email: &created.invitation.email,
      inviter: auth.user.display_name.as_deref().unwrap_or(&auth.user.email),
      account_name: account_name.as_deref(),
      role: &created.invitation.role,
      token: &created.token,
      code: &created.code,
    },
  )
  .await;

  Ok(Json(CreateInvitationResponse {
    invitation_id: created.invitation.id.clone(),
    code: created.code,
    join_url: format!("/join?token={}", created.token),
    email: created.invitation.email,
    role: created.invitation.role,
    expires_at: created.invitation.expires_at,
  }))
}
