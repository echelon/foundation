//! Shared SSO proof verification: turns client-supplied proof (a Google GIS credential, or a
//! GitHub authorization code) into a VERIFIED identity. Used by both the login callback and
//! the link-while-authenticated flow — nothing identity-shaped is ever client-trusted.

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// A cryptographically verified provider identity.
pub struct VerifiedSsoIdentity {
  pub provider: &'static str,
  pub sub: String,
  pub email: String,
  pub display_name: Option<String>,
}

/// The client-supplied proof material (which field applies depends on the provider).
pub struct SsoProof<'a> {
  /// Google: the GIS ID-token JWT.
  pub credential: Option<&'a str>,
  /// GitHub: the authorization code from the web application flow.
  pub code: Option<&'a str>,
  /// GitHub: the PKCE verifier for that code.
  pub code_verifier: Option<&'a str>,
}

pub async fn verify_sso_identity(
  state: &AppState,
  provider: &str,
  proof: SsoProof<'_>,
) -> ApiResult<VerifiedSsoIdentity> {
  match provider {
    "google" => {
      let google = state
        .google
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Google sign-in is not configured on this server".into()))?;
      let credential =
        proof.credential.ok_or_else(|| ApiError::BadRequest("`credential` is required for google".into()))?;
      let id = google.verify(credential).await.map_err(|e| {
        tracing::warn!(error = %e, "google credential rejected");
        ApiError::Unauthorized("Google credential could not be verified".into())
      })?;
      // We mark SSO emails verified (accounts.md §3B) — so only accept ones Google vouches for.
      if !id.email_verified {
        return Err(ApiError::Unauthorized("Google reports this email as unverified".into()));
      }
      Ok(VerifiedSsoIdentity { provider: "google", sub: id.sub, email: id.email, display_name: id.display_name })
    }
    "github" => {
      let github = state
        .github
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("GitHub sign-in is not configured on this server".into()))?;
      let code = proof.code.ok_or_else(|| ApiError::BadRequest("`code` is required for github".into()))?;
      // resolve_code enforces a VERIFIED email via /user/emails (accounts.md §3C).
      let id = github.resolve_code(code, proof.code_verifier).await.map_err(|e| {
        tracing::warn!(error = %e, "github code rejected");
        ApiError::Unauthorized("GitHub sign-in could not be verified".into())
      })?;
      Ok(VerifiedSsoIdentity { provider: "github", sub: id.sub, email: id.email, display_name: id.display_name })
    }
    other => Err(ApiError::BadRequest(format!("unsupported provider: {other}"))),
  }
}
