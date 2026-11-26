use axum::{
    extract::{Form, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use base64::{engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}, Engine};
use db_queries::gateway_mcp::{self, CodeExchange, Grant, Revocation, TokenRotation};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::settings::{Client, Settings};
use super::SharedSettings;
use crate::{auth::service::sha256_hex, error::ApiError, state::AppState};

pub(super) const SCOPES: [&str; 4] = [
    "requests:read", "requests:submit", "requests:cancel", "usage:read",
];

pub(super) async fn metadata(Extension(settings): Extension<SharedSettings>) -> Response {
    #[derive(Serialize)]
    struct Resource<'a> {
        resource: &'a str,
        authorization_servers: [&'a str; 1],
        scopes_supported: [&'static str; 4],
        bearer_methods_supported: [&'static str; 1],
    }
    Json(Resource {
        resource: &settings.resource,
        authorization_servers: [&settings.origin],
        scopes_supported: SCOPES,
        bearer_methods_supported: ["header"],
    }).into_response()
}

pub(super) async fn issuer(Extension(settings): Extension<SharedSettings>) -> Response {
    #[derive(Serialize)]
    struct Issuer<'a> {
        issuer: &'a str,
        authorization_endpoint: String,
        token_endpoint: String,
        revocation_endpoint: String,
        authorization_response_iss_parameter_supported: bool,
        response_types_supported: [&'static str; 1],
        grant_types_supported: [&'static str; 2],
        code_challenge_methods_supported: [&'static str; 1],
        token_endpoint_auth_methods_supported: [&'static str; 3],
        scopes_supported: [&'static str; 4],
    }
    Json(Issuer {
        issuer: &settings.origin,
        authorization_endpoint: format!("{}/mcp/oauth/authorize", settings.origin),
        token_endpoint: format!("{}/mcp/oauth/token", settings.origin),
        revocation_endpoint: format!("{}/mcp/oauth/revoke", settings.origin),
        authorization_response_iss_parameter_supported: true,
        response_types_supported: ["code"],
        grant_types_supported: ["authorization_code", "refresh_token"],
        code_challenge_methods_supported: ["S256"],
        token_endpoint_auth_methods_supported: ["none", "client_secret_post", "client_secret_basic"],
        scopes_supported: SCOPES,
    }).into_response()
}

pub(super) async fn authenticate(
    state: &AppState, settings: &Settings, headers: &HeaderMap,
) -> Result<Grant, Response> {
    let unauthorized = || challenge(settings, None);
    let token = headers.get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.split_once(' '))
        .filter(|(scheme, token)| scheme.eq_ignore_ascii_case("Bearer") && valid_token(token))
        .filter(|_| headers.get_all(header::AUTHORIZATION).iter().count() == 1)
        .map(|(_, token)| token)
        .ok_or_else(unauthorized)?;
    let grant = gateway_mcp::find_access(&state.pool, &sha256_hex(token), &settings.resource)
        .await.map_err(|error| ApiError::from(error).into_response())?
        .ok_or_else(unauthorized)?;
    settings.client(&grant.oauth_client_id).ok_or_else(unauthorized)?;
    Ok(grant)
}

pub(super) fn challenge(settings: &Settings, missing: Option<&str>) -> Response {
    let (status, error, scope) = match missing {
        Some(scope) => (StatusCode::FORBIDDEN, "insufficient_scope", format!("{} {scope}", SCOPES[0])),
        None => (StatusCode::UNAUTHORIZED, "invalid_token", SCOPES[0].to_owned()),
    };
    let mut response = failure(status, error);
    let value = format!(
        "Bearer resource_metadata=\"{}\", scope=\"{}\", error=\"{}\"", settings.metadata, scope, error,
    );
    if let Ok(value) = HeaderValue::from_str(&value) {
        response.headers_mut().insert(header::WWW_AUTHENTICATE, value);
    }
    response
}

#[derive(Deserialize)]
pub(super) struct Exchange {
    grant_type: String,
    resource: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

pub(super) async fn token(
    State(state): State<AppState>,
    Extension(settings): Extension<SharedSettings>,
    headers: HeaderMap,
    Form(form): Form<Exchange>,
) -> Response {
    let Some(client) = client(&settings, &headers, form.client_id.as_deref(), form.client_secret.as_deref()) else {
        return invalid_client();
    };
    if form.resource.as_deref().is_some_and(|resource| resource != settings.resource) {
        return failure(StatusCode::BAD_REQUEST, "invalid_target");
    }
    let access = generate_token();
    let refresh = generate_token();
    let access_hash = sha256_hex(&access);
    let refresh_hash = sha256_hex(&refresh);
    let rotation = TokenRotation {
        previous_hash: "", access_hash: &access_hash, refresh_hash: &refresh_hash,
        client: &client.client_id, resource: &settings.resource,
    };
    let invalid_grant = || failure(StatusCode::BAD_REQUEST, "invalid_grant");
    let result = match form.grant_type.as_str() {
        "authorization_code" => {
            let Some(code) = form.code.filter(|code| valid_token(code)) else { return invalid_grant() };
            let Some(verifier) = form.code_verifier.filter(|verifier| valid_verifier(verifier)) else { return invalid_grant() };
            let Some(redirect) = form.redirect_uri.filter(|uri| client.redirect_uris.contains(uri)) else { return invalid_grant() };
            let hash = sha256_hex(&code);
            gateway_mcp::exchange_code(&state.pool, CodeExchange {
                rotation: TokenRotation { previous_hash: &hash, ..rotation }, redirect_uri: &redirect,
                challenge: &URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
            }).await
        }
        "refresh_token" => {
            // The scope set belongs to the grant. Refresh cannot change it.
            if form.scope.as_deref().is_some_and(|value| !value.is_empty()) {
                return failure(StatusCode::BAD_REQUEST, "invalid_scope");
            }
            let Some(token) = form.refresh_token.filter(|value| valid_token(value)) else { return invalid_grant() };
            let hash = sha256_hex(&token);
            gateway_mcp::rotate_token(&state.pool, TokenRotation { previous_hash: &hash, ..rotation }).await
        }
        _ => return failure(StatusCode::BAD_REQUEST, "unsupported_grant_type"),
    };
    match result {
        Ok(Some(scopes)) => {
            #[derive(Serialize)]
            struct Token {
                access_token: String,
                refresh_token: String,
                token_type: &'static str,
                expires_in: u16,
                scope: String,
            }
            Json(Token {
                access_token: access,
                refresh_token: refresh,
                token_type: "Bearer",
                expires_in: 900,
                scope: scopes.join(" "),
            }).into_response()
        }
        Ok(None) => invalid_grant(),
        Err(error) => ApiError::from(error).into_response(),
    }
}

#[derive(Deserialize)]
pub(super) struct Revoke {
    client_id: Option<String>,
    client_secret: Option<String>,
    token: String,
}

pub(super) async fn revoke(
    State(state): State<AppState>,
    Extension(settings): Extension<SharedSettings>,
    headers: HeaderMap,
    Form(form): Form<Revoke>,
) -> Response {
    let Some(client) = client(&settings, &headers, form.client_id.as_deref(), form.client_secret.as_deref()) else {
        return invalid_client();
    };
    if valid_token(&form.token) {
        if let Err(error) = gateway_mcp::revoke_token(&state.pool, Revocation {
            hash: &sha256_hex(&form.token), client: &client.client_id, resource: &settings.resource,
        }).await {
            return ApiError::from(error).into_response();
        }
    }
    StatusCode::OK.into_response()
}

fn client<'a>(
    settings: &'a Settings, headers: &HeaderMap, id: Option<&str>, secret: Option<&str>,
) -> Option<&'a Client> {
    if headers.get_all(header::AUTHORIZATION).iter().count() > 1 {
        return None;
    }
    if let Some(header) = headers.get(header::AUTHORIZATION) {
        if id.is_some() || secret.is_some() {
            return None;
        }
        let (scheme, encoded) = header.to_str().ok()?.split_once(' ')?;
        if !scheme.eq_ignore_ascii_case("Basic") || encoded.len() > 2048 {
            return None;
        }
        let bytes = STANDARD.decode(encoded).ok()?;
        let (encoded_id, encoded_secret) = std::str::from_utf8(&bytes).ok()?.split_once(':')?;
        // OAuth Basic authentication uses form-encoded client credentials.
        let pair = format!("{encoded_id}={encoded_secret}");
        let mut parsed = reqwest::Url::parse("http://localhost").ok()?;
        parsed.set_query(Some(&pair));
        let mut fields = parsed.query_pairs();
        let (id, secret) = fields.next()?;
        let client = settings.client(&id)?;
        return (fields.next().is_none() && client.accepts_secret(Some(&secret))).then_some(client);
    }
    let client = settings.client(id?)?;
    client.accepts_secret(secret).then_some(client)
}

fn invalid_client() -> Response {
    let mut response = failure(StatusCode::UNAUTHORIZED, "invalid_client");
    response.headers_mut().insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Basic"));
    response
}

pub(super) fn generate_token() -> String {
    let mut bytes = [0; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(super) fn valid_token(value: &str) -> bool {
    value.len() == 43 && URL_SAFE_NO_PAD.decode(value).is_ok_and(|bytes| bytes.len() == 32)
}

fn valid_verifier(value: &str) -> bool {
    (43..=128).contains(&value.len())
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(&byte))
}

pub(super) fn failure(status: StatusCode, error: &'static str) -> Response {
    #[derive(Serialize)]
    struct Error {
        error: &'static str,
    }
    (status, Json(Error { error })).into_response()
}
