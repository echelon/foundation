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
  /// Object storage (R2). `None` when unconfigured — avatar uploads/serving from the bucket are
  /// disabled (uploads error clearly; gravatar display falls back to proxying).
  pub bucket: Option<bucket_client::BucketClient>,
}
