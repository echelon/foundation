use std::collections::HashSet;

use anyhow::{ensure, Context};
use reqwest::Url;
use ring::hmac;
use serde::Deserialize;

use crate::auth::service::sha256_hex;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Client {
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    client_secret_sha256: Option<String>,
}

impl Client {
    pub fn accepts_secret(&self, supplied: Option<&str>) -> bool {
        match (&self.client_secret_sha256, supplied) {
            (None, None) => true,
            (Some(expected), Some(secret)) if secret.len() <= 512 => {
                let supplied = hmac::Key::new(hmac::HMAC_SHA256, sha256_hex(secret).as_bytes());
                let expected = hmac::Key::new(hmac::HMAC_SHA256, expected.as_bytes());
                let message = b"TokenCraft MCP client authentication";
                hmac::verify(&expected, message, hmac::sign(&supplied, message).as_ref()).is_ok()
            }
            _ => false,
        }
    }
}

pub(in crate::gateway_demo) struct Settings {
    pub origin: String,
    pub resource: String,
    pub metadata: String,
    clients: Vec<Client>,
    pub quote_key: hmac::Key,
}

impl Settings {
    pub fn from_env(session_secret: &str) -> anyhow::Result<Option<Self>> {
        let Some(origin) = std::env::var_os("TOKENCRAFT_MCP_ORIGIN") else {
            return Ok(None);
        };
        let origin = origin.into_string().map_err(|_| anyhow::anyhow!("invalid TokenCraft MCP origin"))?;
        let url = Url::parse(&origin).context("invalid TokenCraft MCP origin")?;
        ensure!(
            safe_url(&url) && url.origin().ascii_serialization() == origin
                && url.path() == "/" && url.query().is_none(),
            "TOKENCRAFT_MCP_ORIGIN must be a canonical HTTPS origin or loopback HTTP origin"
        );
        let clients: Vec<Client> = serde_json::from_str(
            &std::env::var("TOKENCRAFT_MCP_CLIENTS").context("TOKENCRAFT_MCP_CLIENTS is required")?,
        )
        .map_err(|_| anyhow::anyhow!("invalid TOKENCRAFT_MCP_CLIENTS JSON"))?;
        let mut seen = HashSet::new();
        ensure!(!clients.is_empty() && clients.len() <= 16, "configure 1 to 16 MCP clients");
        for client in &clients {
            ensure!(
                (1..=128).contains(&client.client_id.len())
                    && client.client_id.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || b"._-".contains(&byte)
                    })
                    && seen.insert(client.client_id.as_str())
                    && (1..=100).contains(&client.name.len())
                    && (1..=8).contains(&client.redirect_uris.len()),
                "invalid or duplicate MCP client"
            );
            if let Some(digest) = &client.client_secret_sha256 {
                ensure!(
                    digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())
                        && digest == &digest.to_ascii_lowercase(),
                    "MCP client secret digest must be lowercase SHA-256 hex"
                );
            }
            for redirect in &client.redirect_uris {
                ensure!(
                    redirect.len() <= 1024 && Url::parse(redirect).is_ok_and(|url| {
                        safe_url(&url) && !url.query_pairs().any(|(key, _)| {
                            matches!(key.as_ref(), "code" | "state" | "iss" | "error")
                        })
                    }),
                    "MCP redirect URI must use HTTPS or loopback HTTP and have no fragment or user info"
                );
            }
        }
        let root_key = hmac::Key::new(hmac::HMAC_SHA256, session_secret.as_bytes());
        let quote_key = hmac::Key::new(
            hmac::HMAC_SHA256,
            hmac::sign(&root_key, b"TokenCraft MCP quote binding").as_ref(),
        );
        Ok(Some(Self {
            resource: format!("{origin}/mcp"),
            metadata: format!("{origin}/.well-known/oauth-protected-resource/mcp"),
            origin,
            clients,
            quote_key,
        }))
    }

    pub(super) fn client(&self, client_id: &str) -> Option<&Client> {
        self.clients.iter().find(|client| client.client_id == client_id)
    }
}

fn safe_url(url: &Url) -> bool {
    let Some(host) = url.host_str() else { return false };
    let local = host == "localhost"
        || host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback());
    (url.scheme() == "https" || url.scheme() == "http" && local)
        && url.username().is_empty() && url.password().is_none() && url.fragment().is_none()
}
