//! The session-auth extractor (auth.md §1, §5). Any handler that takes an [`AuthContext`]
//! argument is authenticated: the middleware verifies the JWT signature, then ONLINE-CHECKS
//! the `sessions` row and the user's `session_version`, so revocation is immediate.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::Utc;

use db_queries::{sessions, users, Session, User};
use ids::{SessionId, UserId};

use crate::auth::cookie::read_session_token;
use crate::error::ApiError;
use crate::state::AppState;

/// The resolved caller — present only when a valid, live session cookie was supplied.
#[derive(Debug, Clone)]
pub struct AuthContext {
  pub user: User,
  pub session: Session,
}

impl AuthContext {
  /// True when this is a staff impersonation session (auth.md §6).
  pub fn is_impersonation(&self) -> bool {
    self.session.impersonator_user_id.is_some()
  }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthContext {
  type Rejection = ApiError;

  async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
    let token = read_session_token(&parts.headers)
      .ok_or_else(|| ApiError::Unauthorized("authentication required".into()))?;

    // 1. Signature + expiry (cheap, no DB).
    let claims = state
      .jwt
      .verify(&token)
      .ok_or_else(|| ApiError::Unauthorized("invalid session".into()))?;

    // 2. Online check: the session row is the revocation authority. The claim ids were minted
    // by us, so they're trusted (no re-validation needed).
    let session_id = SessionId::from_trusted(claims.sid);
    let session = sessions::find_session_by_id(&state.pool, &session_id)
      .await?
      .ok_or_else(|| ApiError::Unauthorized("session not found".into()))?;
    if session.revoked_at.is_some() {
      return Err(ApiError::Unauthorized("session revoked".into()));
    }
    if session.expires_at <= Utc::now() {
      return Err(ApiError::Unauthorized("session expired".into()));
    }

    // 3. session_version vector clock: a stale sv means "logged out everywhere".
    let user_id = UserId::from_trusted(claims.uid);
    let user = users::find_user_by_id(&state.pool, &user_id)
      .await?
      .ok_or_else(|| ApiError::Unauthorized("user not found".into()))?;
    if user.session_version != claims.sv {
      return Err(ApiError::Unauthorized("session superseded".into()));
    }

    Ok(AuthContext { user, session })
  }
}
