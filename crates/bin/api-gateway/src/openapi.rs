//! The gateway's OpenAPI document. Every handler's `#[utoipa::path]` is registered here —
//! adding a handler means adding one line to `paths(...)` — and the `export-openapi` bin
//! prints the document for the static docs build (`just api-docs`).

use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::handlers::{
  account, api_keys, audit_logs, auth, avatar, catalog, files, generation, invitations, login_methods, org, root,
  sessions, webhooks,
};

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
    root::api_status_handler,
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
    login_methods::list_login_methods_handler,
    login_methods::link_sso_handler,
    login_methods::unlink_sso_handler,
    sessions::list_sessions_handler,
    sessions::revoke_other_sessions_handler,
    sessions::revoke_session_handler,
    avatar::upload_my_avatar_handler,
    avatar::set_my_avatar_preference_handler,
    avatar::set_monogram_gradient_handler,
    avatar::remove_my_avatar_handler,
    avatar::remove_my_gravatar_handler,
    avatar::refresh_my_gravatar_handler,
    org::list_my_accounts_handler,
    org::list_account_members_handler,
    org::list_account_users_handler,
    org::remove_account_member_handler,
    org::update_business_name_handler,
    api_keys::list_api_keys_handler,
    api_keys::create_api_key_handler,
    api_keys::delete_api_key_handler,
    audit_logs::list_my_audit_logs_handler,
    audit_logs::list_account_audit_logs_handler,
    audit_logs::list_member_audit_logs_handler,
    invitations::create_invitation_handler,
    invitations::list_account_invitations_handler,
    invitations::cancel_invitation_handler,
    invitations::resolve_invitation_handler,
    invitations::accept_invitation_handler,
    generation::generate_any_image_handler,
    generation::cost_any_image_handler,
    generation::generate_with_model_handler,
    generation::cost_with_model_handler,
    generation::list_jobs_handler,
    generation::get_job_handler,
    generation::get_job_status_handler,
    generation::get_job_request_handler,
    generation::admin_list_jobs_handler,
    generation::admin_get_job_request_handler,
    files::upload_file_handler,
    webhooks::openai_webhook_handler,
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
    (name = "api-keys", description = "API keys: list, mint (secret shown once), revoke"),
    (name = "invitations", description = "Inviting people into an organization"),
    (name = "audit-logs", description = "Keyset-paginated audit trails"),
    (name = "generation", description = "Image generation: enqueue (Any + model-specific planes), cost quotes, job polling"),
    (name = "files", description = "Input uploads: `file_…` ids + CDN URLs for generation inputs"),
    (name = "admin", description = "Staff-only record browsing"),
    (name = "webhooks", description = "Inbound provider webhooks (signature-verified, logged, idempotent)"),
  )
)]
pub struct ApiDoc;

/// Registers the `session_cookie` scheme referenced by `security(("session_cookie" = []))`
/// and the `api_key` scheme (`Authorization: Bearer foundation_api_…`, api_overview.md §auth).
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
    components.add_security_scheme(
      "api_key",
      SecurityScheme::Http(
        utoipa::openapi::security::HttpBuilder::new()
          .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
          .description(Some(
            "Account-scoped API key (`foundation_api_…`) — canonical form `Authorization: Bearer`; \
             `X-API-Key`, `Authorization: Key`, bare `Authorization`, and Basic (key as username) \
             are also accepted.",
          ))
          .build(),
      ),
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
    // 57 operations over 53 paths (sessions, avatar, api_keys, and invitations each pair two on one path).
    assert_eq!(documented.len(), 53, "unexpected number of documented paths: {documented:#?}");
  }
}
