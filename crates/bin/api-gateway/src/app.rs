//! Router assembly + server bootstrap. `main.rs` stays thin over [`run`].

use std::sync::Arc;

use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Method};
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::token::JwtKeys;
use crate::handlers::{account_handlers, auth_handlers, catalog_handlers, invitation_handlers, root_handlers};
use crate::state::AppState;

/// Connect the DB, build state + router, and serve until shutdown.
pub async fn run(cfg: config::Config) -> anyhow::Result<()> {
  let pool = db::connect(&cfg.database_url).await?;
  tracing::info!("connected to database");

  let state = AppState {
    pool,
    jwt: Arc::new(JwtKeys::new(&cfg.session_secret)),
    cookie_domain: cfg.cookie_domain.clone(),
    cookie_secure: cfg.cookie_secure,
    app_base_url: cfg.app_base_url.clone(),
    email_from: cfg.email.from_address.clone(),
    resend_api_key: cfg
      .email
      .resend_api_key
      .clone()
      .map(|k| Arc::new(email_client::ResendApiKey::new(k))),
  };
  if state.resend_api_key.is_none() {
    tracing::warn!("RESEND_API_KEY not set — outbound email will be logged and skipped");
  }

  let app = build_router(state);
  let addr = format!("0.0.0.0:{}", cfg.api_gateway_port);
  let listener = tokio::net::TcpListener::bind(&addr).await?;
  tracing::info!("api-gateway listening on http://{addr}");
  axum::serve(listener, app).await?;
  Ok(())
}

/// The product router group (auth.md §5). Split-seam kept clean so console/admin/webhooks-in
/// groups can mount here later.
pub fn build_router(state: AppState) -> Router {
  // Browser calls come from the frontend origin (credentials/cookies included), so CORS must name
  // origins explicitly — wildcard + credentials is forbidden by the spec. We allow the configured
  // app plus the local dev servers.
  let origins: Vec<HeaderValue> = [
    state.app_base_url.trim_end_matches('/'),
    "http://localhost:3000",
    "http://127.0.0.1:3000",
  ]
  .iter()
  .filter_map(|o| o.parse().ok())
  .collect();
  let cors = CorsLayer::new()
    .allow_origin(origins)
    .allow_credentials(true)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([CONTENT_TYPE]);

  Router::new()
    // hello, human (embedded landing page)
    .route("/", get(root_handlers::index))
    // health + public catalog
    .route("/health", get(catalog_handlers::health))
    .route("/v1/models", get(catalog_handlers::list_models))
    .route("/v1/models/:slug", get(catalog_handlers::get_model))
    // auth
    .route("/v1/auth/signup", post(auth_handlers::signup))
    .route("/v1/auth/login", post(auth_handlers::login))
    .route("/v1/auth/logout", post(auth_handlers::logout))
    .route("/v1/auth/me", get(auth_handlers::me))
    .route("/v1/auth/oauth/:provider/callback", post(auth_handlers::oauth_callback))
    // account security: password, email, forgot/reset, sessions
    .route("/v1/auth/password", put(account_handlers::change_password))
    .route("/v1/auth/email", put(account_handlers::change_email))
    .route("/v1/auth/password-reset", post(account_handlers::request_password_reset))
    .route("/v1/auth/password-reset/confirm", post(account_handlers::confirm_password_reset))
    .route(
      "/v1/auth/sessions",
      get(account_handlers::list_sessions).delete(account_handlers::revoke_sessions),
    )
    .route("/v1/auth/sessions/:id", delete(account_handlers::revoke_session))
    // accounts / invitations
    .route("/v1/accounts/:account_id/invitations", post(invitation_handlers::create_invitation))
    .route("/v1/invitations/:code_or_token", get(invitation_handlers::resolve_invitation))
    .route("/v1/invitations/:id/accept", post(invitation_handlers::accept_invitation))
    .layer(cors)
    .layer(TraceLayer::new_for_http())
    .with_state(state)
}
