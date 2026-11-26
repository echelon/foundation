use std::fmt::Write as _;

use axum::{
    extract::{Form, OriginalUri, Query, State},
    http::{header, HeaderMap, StatusCode, Uri},
    response::{Html, IntoResponse, Redirect, Response},
    Extension,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use db_queries::{gateway_mcp::{self, NewGrant}, members};
use ids::{AccountId, GatewayMcpGrantId, GatewayProjectId};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::{oauth, settings::{Client, Settings}, SharedSettings};
use super::super::access::{self, Permission};
use crate::{auth::service::sha256_hex, error::{ApiError, ApiResult}, middleware::AuthContext, state::AppState};

#[derive(Deserialize, Serialize)]
pub(super) struct Authorization {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    resource: Option<String>,
    scope: Option<String>,
    code_challenge: String,
    code_challenge_method: String,
}

impl Authorization {
    fn client<'a>(&self, settings: &'a Settings) -> ApiResult<&'a Client> {
        let client = settings.client(&self.client_id)
            .filter(|client| client.redirect_uris.contains(&self.redirect_uri))
            .ok_or_else(|| ApiError::BadRequest("unknown OAuth client or redirect URI".into()))?;
        if self.response_type != "code" || self.code_challenge_method != "S256"
            || !oauth::valid_token(&self.code_challenge)
            || self.state.as_ref().is_some_and(|state| state.len() > 1024)
            || self.resource.as_deref().is_some_and(|resource| resource != settings.resource)
        {
            return Err(ApiError::BadRequest("invalid OAuth authorization request".into()));
        }
        self.eligible_scopes()?;
        Ok(client)
    }

    fn eligible_scopes(&self) -> ApiResult<Vec<&str>> {
        let Some(scope) = self.scope.as_deref() else { return Ok(oauth::SCOPES.to_vec()) };
        let scopes: Vec<&str> = scope.split_ascii_whitespace().collect();
        if scopes.len() > oauth::SCOPES.len() || !scopes.contains(&oauth::SCOPES[0])
            || scopes.iter().any(|scope| !oauth::SCOPES.contains(scope))
        {
            return Err(ApiError::BadRequest("invalid OAuth scopes; requests:read is required".into()));
        }
        Ok(scopes)
    }

    fn redirect(&self, settings: &Settings, key: &str, value: &str) -> ApiResult<Redirect> {
        let mut url = Url::parse(&self.redirect_uri)
            .map_err(|_| ApiError::Internal("invalid registered redirect URI".into()))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair(key, value).append_pair("iss", &settings.origin);
            if let Some(state) = &self.state {
                query.append_pair("state", state);
            }
        }
        Ok(Redirect::to(url.as_str()))
    }
}

pub(super) async fn show(
    State(state): State<AppState>,
    Extension(settings): Extension<SharedSettings>,
    auth: Result<AuthContext, ApiError>,
    OriginalUri(uri): OriginalUri,
    Query(request): Query<Authorization>,
) -> ApiResult<Response> {
    let client = request.client(&settings)?;
    let Some(auth) = browser_user(auth)? else {
        return login_redirect(&state, &uri);
    };
    let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&request)
        .map_err(|_| ApiError::Internal("could not prepare OAuth consent".into()))?);
    let mut form = format!(
        "<p><strong>{}</strong> is requesting access to your TokenCraft project.</p>\
         <form method=post><input type=hidden name=request value=\"{encoded}\">\
         <label for=project>Project</label><select id=project name=project required>",
        escape(&client.name),
    );
    let mut options = 0;
    for account in members::list_memberships_with_accounts(&state.pool, &auth.user.id).await? {
        let name = account.business_name.as_deref().unwrap_or(&account.placeholder_business_name);
        let projects = access::session_projects(&state.pool, &auth, &account.account_id, None, false).await?;
        for project in projects {
            if access::role_allows(&project.role, Permission::ReadRequests) {
                let client_scope = project.client_ref.as_deref().map_or(String::new(), |client| format!(" · client {}", escape(client)));
                write!(form, "<option value=\"{}/{}\">{} / {}{client_scope}</option>",
                    account.account_id, project.id, escape(name), escape(&project.name)).unwrap();
                options += 1;
            }
        }
    }
    form.push_str("</select><p>Read requests, results and traces within the selected project.</p>");
    let scopes = request.eligible_scopes()?;
    for (scope, name, label) in [
        ("requests:submit", "submit", "Allow generation submissions and inference charges"),
        ("requests:cancel", "cancel", "Allow cancellation of queued work"),
        ("usage:read", "usage", "Allow usage and cost reports"),
    ] {
        if scopes.contains(&scope) {
            write!(form, "<label><input type=checkbox name={name} value=yes> {label}</label>").unwrap();
        }
    }
    if options > 0 {
        form.push_str("<button name=decision value=allow>Connect TokenCraft</button>");
    } else {
        form.push_str("<p>No project currently permits request reads for this account.</p>");
    }
    form.push_str("<button name=decision value=deny formnovalidate>Decline</button></form>");
    Ok(page("Connect TokenCraft", &form))
}

#[derive(Deserialize)]
pub(super) struct Consent {
    request: String,
    project: Option<String>,
    decision: String,
    submit: Option<String>,
    cancel: Option<String>,
    usage: Option<String>,
}

pub(super) async fn submit(
    State(state): State<AppState>,
    Extension(settings): Extension<SharedSettings>,
    auth: AuthContext,
    headers: HeaderMap,
    Form(form): Form<Consent>,
) -> ApiResult<Response> {
    require_browser_post(&settings, &auth, &headers)?;
    let request: Authorization = URL_SAFE_NO_PAD.decode(&form.request).ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .ok_or_else(|| ApiError::BadRequest("invalid OAuth consent".into()))?;
    let client = request.client(&settings)?;
    if form.decision == "deny" {
        return Ok(request.redirect(&settings, "error", "access_denied")?.into_response());
    }
    if form.decision != "allow" {
        return Err(ApiError::BadRequest("select a consent decision".into()));
    }
    let (account, project) = form.project.as_deref().and_then(|value| value.split_once('/'))
        .ok_or_else(|| ApiError::BadRequest("select a project".into()))?;
    let account: AccountId = access::parse_id(account, "account_id")?;
    let project: GatewayProjectId = access::parse_id(project, "project_id")?;
    let mut entries = access::session_projects(&state.pool, &auth, &account, Some(&project), false).await?;
    let entry = entries.pop().ok_or_else(|| ApiError::NotFound("project not found".into()))?;
    let requested = request.eligible_scopes()?;
    let mut scopes = Vec::new();
    for (permission, enable) in [
        (Permission::ReadRequests, true),
        (Permission::SubmitRequests, form.submit.as_deref() == Some("yes")),
        (Permission::CancelRequests, form.cancel.as_deref() == Some("yes")),
        (Permission::ReadUsage, form.usage.as_deref() == Some("yes")),
    ] {
        if enable {
            if !requested.contains(&permission.as_str()) || !access::role_allows(&entry.role, permission) {
                return Err(ApiError::Forbidden(format!("project role or connector request does not allow {}", permission.as_str())));
            }
            scopes.push(permission.as_str().to_owned());
        }
    }
    let code = oauth::generate_token();
    gateway_mcp::create_grant(&state.pool, NewGrant {
        id: &GatewayMcpGrantId::generate(),
        user: &auth.user.id,
        session_version: auth.user.session_version,
        account: &account,
        project: &project,
        client_ref: entry.client_ref.as_deref(),
        client: &client.client_id,
        redirect_uri: &request.redirect_uri,
        resource: &settings.resource,
        scopes: &scopes,
        challenge: &request.code_challenge,
        code_hash: &sha256_hex(&code),
    }).await?;
    Ok(request.redirect(&settings, "code", &code)?.into_response())
}

pub(super) async fn connections(
    State(state): State<AppState>,
    Extension(settings): Extension<SharedSettings>,
    auth: Result<AuthContext, ApiError>,
    OriginalUri(uri): OriginalUri,
) -> ApiResult<Response> {
    let Some(auth) = browser_user(auth)? else {
        return login_redirect(&state, &uri);
    };
    let connections = gateway_mcp::list_grants(&state.pool, &auth.user.id, &settings.resource).await?;
    let mut content = if connections.is_empty() { "<p>No active connections.</p>".into() } else { String::new() };
    for connection in connections {
        let name = settings.client(&connection.oauth_client_id)
            .map_or(connection.oauth_client_id.as_str(), |client| client.name.as_str());
        write!(content,
            "<form method=post><input type=hidden name=id value=\"{}\">\
             <p><strong>{}</strong> · project {}<br>{}</p><button>Disconnect</button></form>",
            connection.id, escape(name), connection.project_id, escape(&connection.scopes.join(", ")),
        ).unwrap();
    }
    Ok(page("TokenCraft agent connections", &content))
}

#[derive(Deserialize)]
pub(super) struct Disconnect {
    id: String,
}

pub(super) async fn disconnect(
    State(state): State<AppState>,
    Extension(settings): Extension<SharedSettings>,
    auth: AuthContext,
    headers: HeaderMap,
    Form(form): Form<Disconnect>,
) -> ApiResult<Redirect> {
    require_browser_post(&settings, &auth, &headers)?;
    let id = access::parse_id(&form.id, "grant id")?;
    gateway_mcp::revoke_grant(&state.pool, &id, &auth.user.id, &settings.resource).await?;
    Ok(Redirect::to("/mcp/oauth/connections"))
}

fn browser_user(auth: Result<AuthContext, ApiError>) -> ApiResult<Option<AuthContext>> {
    match auth {
        Ok(auth) if !auth.is_impersonation() => Ok(Some(auth)),
        Ok(_) => Err(ApiError::Forbidden("impersonation cannot manage agent connections".into())),
        Err(ApiError::Unauthorized(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

fn require_browser_post(settings: &Settings, auth: &AuthContext, headers: &HeaderMap) -> ApiResult<()> {
    if auth.is_impersonation() || headers.get(header::ORIGIN).and_then(|value| value.to_str().ok()) != Some(settings.origin.as_str()) {
        return Err(ApiError::Forbidden("use the TokenCraft consent page".into()));
    }
    Ok(())
}

fn login_redirect(state: &AppState, uri: &Uri) -> ApiResult<Response> {
    let mut login = Url::parse(&state.app_base_url).and_then(|base| base.join("/login"))
        .map_err(|_| ApiError::Internal("invalid TokenCraft application URL".into()))?;
    let mut bridge = login.join("/connect/agents")
        .map_err(|_| ApiError::Internal("invalid TokenCraft continuation URL".into()))?;
    bridge.query_pairs_mut().append_pair("continue", &uri.to_string());
    let next = format!("{}?{}", bridge.path(), bridge.query().unwrap_or_default());
    login.query_pairs_mut().append_pair("redirect_to", &next);
    Ok(Redirect::to(login.as_str()).into_response())
}

fn page(title: &str, content: &str) -> Response {
    (StatusCode::OK, Html(format!(
        "<!doctype html><html lang=en><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\">\
         <title>{title}</title><style>body{{font:1rem/1.55 system-ui;margin:3rem auto;padding:0 1rem;max-width:36rem}}\
         label,select{{display:block;margin:.7rem 0}}select,button{{font:inherit;padding:.45rem .7rem;max-width:100%}}\
         button{{margin:.8rem .6rem 0 0}}form{{margin:1rem 0 2rem}}</style>\
         <main><h1>{title}</h1>{content}</main></html>"
    ))).into_response()
}

fn escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&#39;")
}
