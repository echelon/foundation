//! `POST /v1/invitations/{id}/accept` — authenticated; creates the membership.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use ids::InvitationId;

use crate::auth::service;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct AcceptInvitationResponse {
  pub ok: bool,
  #[schema(value_type = String, example = "inv_01j9…")]
  pub invitation_id: InvitationId,
}

#[utoipa::path(
  post, path = "/v1/invitations/{id}/accept",
  tag = "invitations",
  security(("session_cookie" = [])),
  params(("id" = String, Path, description = "Invitation id (`inv_…`)")),
  responses(
    (status = 200, description = "Membership created", body = AcceptInvitationResponse),
    (status = 400, description = "Invalid invitation id"),
    (status = 404, description = "Unknown, expired, revoked, or already-accepted invitation")
  )
)]
pub async fn accept_invitation_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(id): Path<String>,
) -> ApiResult<Json<AcceptInvitationResponse>> {
  let invitation_id: InvitationId = id
    .parse()
    .map_err(|_| ApiError::BadRequest("invalid invitation id".into()))?;
  service::accept_invitation(&state.pool, &invitation_id, &auth.user.id).await?;
  Ok(Json(AcceptInvitationResponse { ok: true, invitation_id }))
}
