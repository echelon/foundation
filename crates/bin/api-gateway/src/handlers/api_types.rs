//! Response types shared across handler areas. Area-specific types live next to their
//! handler; single-use types live in the handler's own file.

use serde::Serialize;
use utoipa::ToSchema;

use db_queries::User;
use ids::UserId;

use crate::avatar_links::AvatarInfo;

/// The bare acknowledgement.
#[derive(Debug, Serialize, ToSchema)]
pub struct OkResponse {
  pub ok: bool,
}

impl OkResponse {
  pub fn ok() -> Self {
    Self { ok: true }
  }
}

/// Acknowledgement + a human-readable message.
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
  pub ok: bool,
  pub message: String,
}

/// The user as APIs report it. `avatar` is present on `/me` (omitted from the wire when None,
/// e.g. right after signup/login).
#[derive(Debug, Serialize, ToSchema)]
pub struct UserBody {
  #[schema(value_type = String, example = "user_01j9…")]
  pub id: UserId,
  pub email: String,
  pub display_name: Option<String>,
  pub is_staff: bool,
  pub email_verified: bool,
  /// False for SSO-only users — the password form offers "set" instead of "change".
  pub has_password: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub avatar: Option<AvatarInfo>,
}

impl UserBody {
  pub fn from_user(u: &User) -> Self {
    Self {
      id: u.id.clone(),
      email: u.email.clone(),
      display_name: u.display_name.clone(),
      is_staff: u.is_staff,
      email_verified: u.email_verified_at.is_some(),
      has_password: u.password_hash.is_some(),
      avatar: None,
    }
  }
}
