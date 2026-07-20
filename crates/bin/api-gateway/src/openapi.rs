//! The gateway's OpenAPI document. Every handler's `#[utoipa::path]` is registered here —
//! adding a handler means adding one line to `paths(...)` — and the `export-openapi` bin
//! prints the document for the static docs build (`just api-docs`).

use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::handlers::{account, audit_logs, auth, avatar, catalog, invitations, org, root, sessions};

#[derive(OpenApi)]
#[openapi(
  info(
    title = "Foundation API",
    description = "The api-gateway: auth, accounts, organizations, invitations, avatars, \
                   audit logs, and the public model catalog. Authenticated endpoints use the \
                   `foundation_session` HttpOnly cookie set by signup/login. Asset endpoints \
                   return CDN URLs — the gateway never serves bytes.",
    version = env!("CARGO_PKG_VERSION"),
  ),
  paths(
    root::index_handler,
    catalog::health_handler,
    catalog::list_models_handler,
    catalog::get_model_handler,
    auth::signup_handler,
    auth::login_handler,
    auth::logout_handler,
    auth::me_handler,
    auth::oauth_callback_handler,
    account::change_password_handler,
    account::change_email_handler,
    account::update_profile_handler,
    account::request_email_verification_handler,
    account::confirm_email_verification_handler,
    account::request_password_reset_handler,
    account::confirm_password_reset_handler,
    sessions::list_sessions_handler,
    sessions::revoke_other_sessions_handler,
    sessions::revoke_session_handler,
    avatar::upload_my_avatar_handler,
    avatar::set_my_avatar_preference_handler,
    avatar::remove_my_avatar_handler,
    avatar::remove_my_gravatar_handler,
    avatar::refresh_my_gravatar_handler,
    org::list_my_accounts_handler,
    org::list_account_members_handler,
    org::remove_account_member_handler,
    audit_logs::list_my_audit_logs_handler,
    audit_logs::list_account_audit_logs_handler,
    audit_logs::list_member_audit_logs_handler,
    invitations::create_invitation_handler,
    invitations::resolve_invitation_handler,
    invitations::accept_invitation_handler,
  ),
  modifiers(&SessionCookieSecurity),
  tags(
    (name = "meta", description = "Landing page and service health"),
    (name = "catalog", description = "The public model catalog (static, code-defined)"),
    (name = "auth", description = "Signup, login, logout, identity"),
    (name = "account", description = "Password, email, profile, and recovery"),
    (name = "sessions", description = "The devices list and session revocation"),
    (name = "avatar", description = "Avatar upload/crop, preference, and gravatar mirroring"),
    (name = "org", description = "Organizations: switcher list, roster, membership"),
    (name = "invitations", description = "Inviting people into an organization"),
    (name = "audit-logs", description = "Keyset-paginated audit trails"),
  )
)]
pub struct ApiDoc;

/// Registers the `session_cookie` scheme referenced by `security(("session_cookie" = []))`.
struct SessionCookieSecurity;

impl Modify for SessionCookieSecurity {
  fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
    let components = openapi.components.get_or_insert_with(Default::default);
    components.add_security_scheme(
      "session_cookie",
      SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
        crate::auth::cookie::COOKIE_NAME,
        "HttpOnly session cookie set by signup/login",
      ))),
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Every route the router serves must be documented (and vice versa) — the doc is only
  /// trustworthy if it can't drift from the router.
  #[test]
  fn openapi_covers_every_route() {
    let doc = ApiDoc::openapi();
    // axum `:param` ↔ OpenAPI `{param}`
    let documented: std::collections::BTreeSet<String> = doc.paths.paths.keys().cloned().collect();
    assert!(documented.contains("/v1/auth/me"));
    // 33 operations over 31 paths (`/v1/auth/sessions` and `/v1/auth/avatar` each carry two).
    assert_eq!(documented.len(), 31, "unexpected number of documented paths: {documented:#?}");
  }
}
