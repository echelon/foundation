//! TokenCraft MCP transport, OAuth resource, and typed tools for the scoped gateway.

use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderValue},
    response::Response,
    routing::{get, post},
    Extension, Router,
};

use crate::state::AppState;

mod consent;
mod oauth;
mod protocol;
mod settings;
mod tools;

pub(super) use settings::Settings;
type SharedSettings = Arc<Settings>;

pub(super) fn router(settings: Settings) -> Router<AppState> {
    Router::new()
        .route("/mcp", post(protocol::handle))
        .route("/.well-known/oauth-protected-resource/mcp", get(oauth::metadata))
        .route("/.well-known/oauth-authorization-server", get(oauth::issuer))
        .route("/mcp/oauth/authorize", get(consent::show).post(consent::submit))
        .route("/mcp/oauth/token", post(oauth::token))
        .route("/mcp/oauth/revoke", post(oauth::revoke))
        .route("/mcp/oauth/connections", get(consent::connections).post(consent::disconnect))
        .layer(Extension(Arc::new(settings)))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(response_headers))
}

async fn response_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    // Same-origin browser form submissions need an Origin for the consent check.
    // This policy still omits the referrer on the external OAuth callback.
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("same-origin"));
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(
        "default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'; base-uri 'none'",
    ));
    response
}
