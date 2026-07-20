//! Shared application state handed to every handler (cloned cheaply — the pool is an `Arc`
//! internally, the JWT keys are wrapped in one).

use std::sync::Arc;

use db::PgPool;

use crate::auth::token::JwtKeys;

#[derive(Clone)]
pub struct AppState {
  pub pool: PgPool,
  pub jwt: Arc<JwtKeys>,
  /// Cookie `Domain` attribute.
  pub cookie_domain: String,
  /// Whether to set the `Secure` cookie attribute.
  pub cookie_secure: bool,
}
