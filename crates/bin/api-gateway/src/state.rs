//! Shared application state handed to every handler (cloned cheaply — the pool is an `Arc`
//! internally, the JWT keys are wrapped in one).

use std::sync::Arc;

use db::PgPool;
use email_client::ResendApiKey;

use crate::auth::token::JwtKeys;

#[derive(Clone)]
pub struct AppState {
  pub pool: PgPool,
  pub jwt: Arc<JwtKeys>,
  /// Cookie `Domain` attribute.
  pub cookie_domain: String,
  /// Whether to set the `Secure` cookie attribute.
  pub cookie_secure: bool,
  /// Public base URL of the frontend, used to build emailed links.
  pub app_base_url: String,
  /// Sender in display form for outbound email, e.g. `"Foundation <noreply@…>"`.
  pub email_from: String,
  /// Resend API key. `None` when email is unconfigured — flows that would send just log + no-op
  /// (the request still succeeds), which keeps local dev testable without a key.
  pub resend_api_key: Option<Arc<ResendApiKey>>,
  /// Object storage (R2). `None` when unconfigured — avatar uploads are disabled (they error
  /// clearly) and un-mirrored avatars simply have no links yet.
  pub bucket: Option<bucket_client::BucketClient>,
  /// Public CDN host in front of the bucket (R2 public/dev URL locally). APIs derive asset URLs
  /// from this — the gateway never serves bytes.
  pub cdn_base_url: String,
  /// Which environment we're in — picks the thumbnail-template shape (prod has an image
  /// resizer; dev serves full images).
  pub environment: config::Environment,
  /// Operator-set status/downtime message for `GET /v1/api_status` (frontend banners).
  /// `None` = all quiet.
  pub status_message: Option<String>,
  /// Google ID-token verifier. `None` when GOOGLE_CLIENT_ID is unset — the SSO endpoint
  /// reports Google sign-in as unavailable rather than failing boot.
  pub google: Option<Arc<crate::auth::google::GoogleAuth>>,
  /// GitHub code exchanger. `None` when GITHUB_CLIENT_ID/SECRET are unset — same graceful-off.
  pub github: Option<Arc<crate::auth::github::GithubAuth>>,
  /// OpenAI provider credential for generation dispatch. `None` = OpenAI models undispatchable
  /// (enqueue still works; jobs fail at dispatch with provider_error).
  pub openai: Option<Arc<openai_client::creds::openai_api_key::OpenAiApiKey>>,
}
