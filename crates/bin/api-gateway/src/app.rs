//! Router assembly + server bootstrap. `main.rs` stays thin over [`run`].

use std::sync::Arc;

use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Method};
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::auth::token::JwtKeys;
use crate::handlers::{
  account, api_keys, audit_logs, auth, avatar, catalog, files, generation, invitations, login_methods, org, root,
  sessions, webhooks,
};
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
    bucket: match &cfg.storage {
      Some(s) => Some(
        bucket_client::BucketClientBuilder::new()
          .endpoint(&s.endpoint)
          .access_key(&s.access_key_id)
          .secret_key(&s.secret_access_key)
          .region_name(&s.region)
          .bucket_name(&s.bucket_name)
          .build()
          .map_err(|e| anyhow::anyhow!("bucket client: {e}"))?,
      ),
      None => None,
    },
    cdn_base_url: cfg.cdn_base_url.clone(),
    environment: cfg.environment.clone(),
    status_message: cfg.status_message.clone(),
    google: cfg
      .google_client_id
      .clone()
      .map(|id| Arc::new(crate::auth::google::GoogleAuth::new(id))),
    github: cfg
      .github_oauth
      .clone()
      .map(|g| Arc::new(crate::auth::github::GithubAuth::new(g.client_id, g.client_secret))),
    openai: cfg
      .providers
      .openai
      .clone()
      .map(|k| Arc::new(openai_client::creds::openai_api_key::OpenAiApiKey::new(k))),
    openai_webhook_secret: cfg.providers.openai_webhook_secret.clone().and_then(|s| {
      match openai_client::creds::openai_webhook_secret::OpenAiWebhookSecret::new(s) {
        Ok(secret) => Some(Arc::new(secret)),
        Err(e) => {
          tracing::error!("OPENAI_WEBHOOK_SECRET is malformed ({e}) — webhook receiving disabled");
          None
        }
      }
    }),
  };
  if state.resend_api_key.is_none() {
    tracing::warn!("RESEND_API_KEY not set — outbound email will be logged and skipped");
  }
  if state.bucket.is_none() {
    tracing::warn!("R2 storage not configured — avatar uploads disabled");
  }
  if state.google.is_none() {
    tracing::warn!("GOOGLE_CLIENT_ID not set — Google sign-in disabled");
  }
  if state.github.is_none() {
    tracing::warn!("GITHUB_CLIENT_ID/GITHUB_CLIENT_SECRET not set — GitHub sign-in disabled");
  }
  // Asset URLs are derived from this base — surface what we resolved, and catch the classic
  // dev misconfig: R2 secrets set but CDN base still pointing at the (dead) local-minio default.
  tracing::info!(cdn_base_url = %state.cdn_base_url, "asset links will use this CDN base");
  if state.bucket.is_some()
    && state.environment == config::Environment::Dev
    && state.cdn_base_url.contains("localhost:9000")
  {
    tracing::warn!(
      "R2 is configured but CDN_BASE_URL is still the local-minio default — avatar links will 404. \
       Set CDN_BASE_URL to your bucket's public URL (e.g. https://pub-<hash>.r2.dev) in \
       api-gateway.development-secrets.env"
    );
  }

  if state.openai.is_none() {
    tracing::warn!("OPENAI_API_KEY not set — OpenAI generation dispatch disabled");
  }
  if state.openai_webhook_secret.is_none() {
    tracing::warn!("OPENAI_WEBHOOK_SECRET not set — inbound OpenAI webhooks disabled (polling still completes jobs)");
  }
  // The in-process dispatcher loop (dispatch.md's worker, embedded for now — extractable
  // to its own bin when scale demands).
  crate::generation::dispatcher::spawn(state.clone());

  let app = build_router(state);
  let addr = format!("0.0.0.0:{}", cfg.api_gateway_port);
  let listener = tokio::net::TcpListener::bind(&addr).await?;
  tracing::info!("api-gateway listening on http://{addr}");
  // with_connect_info: gives the trace span a peer address even when no proxy header is set.
  axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
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

  // Every request log line carries method, path, and remote ip. The ip prefers
  // `x-forwarded-for` (Tailscale funnel / any fronting proxy) and falls back to the raw
  // peer address; responses log at INFO so the context is visible with default filters.
  let trace = TraceLayer::new_for_http()
    .make_span_with(|request: &axum::http::Request<axum::body::Body>| {
      let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
          request
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
        })
        .unwrap_or_else(|| "-".to_string());
      tracing::info_span!("http", method = %request.method(), path = %request.uri().path(), ip = %ip)
    })
    .on_response(DefaultOnResponse::new().level(Level::INFO));

  Router::new()
    // hello, human (embedded landing page)
    .route("/", get(root::index_handler))
    // health + public catalog
    .route("/health", get(catalog::health_handler))
    .route("/v1/api_status", get(root::api_status_handler))
    .route("/v1/models", get(catalog::list_models_handler))
    .route("/v1/models/:slug", get(catalog::get_model_handler))
    // auth
    .route("/v1/auth/signup", post(auth::signup_handler))
    .route("/v1/auth/login", post(auth::login_handler))
    .route("/v1/auth/logout", post(auth::logout_handler))
    .route("/v1/auth/me", get(auth::me_handler))
    .route("/v1/auth/oauth/:provider/callback", post(auth::oauth_callback_handler))
    // account security: password, email, forgot/reset, sessions
    .route("/v1/auth/password", put(account::change_password_handler))
    // login methods: the password entry + SSO connections (POST {provider} links, DELETE
    // {sso_id} unlinks — one axum route, two param meanings)
    .route("/v1/auth/login_methods", get(login_methods::list_login_methods_handler))
    .route(
      "/v1/auth/login_methods/:provider_or_id",
      post(login_methods::link_sso_handler).delete(login_methods::unlink_sso_handler),
    )
    .route("/v1/auth/email", put(account::change_email_handler))
    .route("/v1/auth/profile", put(account::update_profile_handler))
    .route("/v1/auth/email/verification", post(account::request_email_verification_handler))
    .route("/v1/auth/email/verification/confirm", post(account::confirm_email_verification_handler))
    .route("/v1/auth/password-reset", post(account::request_password_reset_handler))
    .route("/v1/auth/password-reset/confirm", post(account::confirm_password_reset_handler))
    .route(
      "/v1/auth/sessions",
      get(sessions::list_sessions_handler).delete(sessions::revoke_other_sessions_handler),
    )
    .route("/v1/auth/sessions/:id", delete(sessions::revoke_session_handler))
    // avatar: upload+crop / remove / preference / gravatar remove+refresh. (No GET — clients
    // render the CDN links returned by /me and these mutations; the gateway never serves bytes.)
    .route(
      "/v1/auth/avatar",
      post(avatar::upload_my_avatar_handler).delete(avatar::remove_my_avatar_handler),
    )
    .route("/v1/auth/avatar/preference", put(avatar::set_my_avatar_preference_handler))
    .route("/v1/auth/avatar/monogram", put(avatar::set_monogram_gradient_handler))
    .route("/v1/auth/avatar/gravatar", delete(avatar::remove_my_gravatar_handler))
    .route("/v1/auth/avatar/gravatar/refresh", post(avatar::refresh_my_gravatar_handler))
    // organizations: switcher list, roster, member removal, audit trails
    .route("/v1/accounts", get(org::list_my_accounts_handler))
    .route("/v1/accounts/:account_id/business", put(org::update_business_name_handler))
    // api keys: any member may list/create/delete (auth.md §4, revised 2026-07-21)
    .route(
      "/v1/accounts/:account_id/api_keys",
      get(api_keys::list_api_keys_handler).post(api_keys::create_api_key_handler),
    )
    .route("/v1/accounts/:account_id/api_keys/:key_id", delete(api_keys::delete_api_key_handler))
    .route("/v1/accounts/:account_id/members", get(org::list_account_members_handler))
    .route("/v1/accounts/:account_id/users", get(org::list_account_users_handler))
    .route("/v1/accounts/:account_id/members/:user_id", delete(org::remove_account_member_handler))
    .route("/v1/user/audit_logs", get(audit_logs::list_my_audit_logs_handler))
    .route("/v1/accounts/:account_id/audit_logs", get(audit_logs::list_account_audit_logs_handler))
    .route(
      "/v1/accounts/:account_id/members/:user_id/audit_logs",
      get(audit_logs::list_member_audit_logs_handler),
    )
    // accounts / invitations
    .route(
      "/v1/accounts/:account_id/invitations",
      post(invitations::create_invitation_handler).get(invitations::list_account_invitations_handler),
    )
    .route(
      "/v1/accounts/:account_id/invitations/:invitation_id",
      delete(invitations::cancel_invitation_handler),
    )
    .route("/v1/invitations/:code_or_token", get(invitations::resolve_invitation_handler))
    .route("/v1/invitations/:id/accept", post(invitations::accept_invitation_handler))
    // generation: the Any plane, the unified model plane, their cost twins, job polling
    .route("/v1/any/image", post(generation::generate_any_image_handler))
    .route("/v1/any/image/cost", post(generation::cost_any_image_handler))
    .route("/v1/models/:slug/generate", post(generation::generate_with_model_handler))
    .route("/v1/models/:slug/generate/cost", post(generation::cost_with_model_handler))
    .route("/v1/jobs", get(generation::list_jobs_handler))
    .route("/v1/jobs/:id", get(generation::get_job_handler))
    .route("/v1/jobs/:id/status", get(generation::get_job_status_handler))
    .route("/v1/jobs/:id/request", get(generation::get_job_request_handler))
    .route("/v1/admin/jobs", get(generation::admin_list_jobs_handler))
    .route("/v1/admin/jobs/:id/request", get(generation::admin_get_job_request_handler))
    // input uploads (image editing sources)
    .route("/v1/files", post(files::upload_file_handler))
    // inbound provider webhooks (signature-authed, not session-authed)
    .route("/v1/webhooks/openai", post(webhooks::openai_webhook_handler))
    // Avatar photos and generation input images are whole files; the axum default (2 MB)
    // is far too small.
    .layer(axum::extract::DefaultBodyLimit::max(32 * 1024 * 1024))
    .layer(cors)
    .layer(trace)
    .with_state(state)
}
