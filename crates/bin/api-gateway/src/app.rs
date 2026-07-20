//! Router assembly + server bootstrap. `main.rs` stays thin over [`run`].

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::auth::token::JwtKeys;
use crate::handlers::{auth_handlers, catalog_handlers, invitation_handlers};
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
  };

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
  Router::new()
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
    // accounts / invitations
    .route("/v1/accounts/:account_id/invitations", post(invitation_handlers::create_invitation))
    .route("/v1/invitations/:code_or_token", get(invitation_handlers::resolve_invitation))
    .route("/v1/invitations/:id/accept", post(invitation_handlers::accept_invitation))
    .layer(TraceLayer::new_for_http())
    .with_state(state)
}
