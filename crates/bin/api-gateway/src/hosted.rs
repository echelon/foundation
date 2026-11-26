use std::net::{IpAddr, Ipv4Addr};

use anyhow::{Context, ensure};
use axum::extract::{Request, State};
use axum::http::{HeaderValue, header::ORIGIN};
use axum::middleware::Next;
use axum::response::Response;
use config::Config;
use reqwest::Url;

use crate::{auth::cookie, error::ApiError};

pub(crate) fn configure(cfg: &mut Config) -> anyhow::Result<IpAddr> {
  db::verify_hosted_url(&cfg.database_url)?;
  verify_origin(&cfg.app_base_url)?;
  ensure!(cfg.cookie_secure, "the hosted gateway requires COOKIE_SECURE=true");
  strong_session_secret(&cfg.session_secret)?;
  let payload_key = std::env::var("TOKENCRAFT_DEMO_PAYLOAD_KEY").unwrap_or_default();
  ensure!(
    !cfg.session_secret.eq_ignore_ascii_case(&payload_key),
    "the hosted session secret and payload key must be independent"
  );
  if let Some(port) = std::env::var_os("API_GATEWAY_PORT") {
    ensure!(
      port.to_str().and_then(|value| value.parse::<u16>().ok()).is_some_and(|port| port > 0),
      "API_GATEWAY_PORT must be a positive port number"
    );
  }
  // The browser sends this cookie only to the gateway host, including across same-site origins.
  cfg.cookie_domain.clear();
  let ip = std::env::var_os("TOKENCRAFT_LISTEN_IP");
  match ip {
    None => Ok(Ipv4Addr::LOCALHOST.into()),
    Some(ip) => ip.to_str().and_then(|value| value.parse().ok())
      .context("TOKENCRAFT_LISTEN_IP must be a literal IPv4 or IPv6 address"),
  }
}

fn verify_origin(raw: &str) -> anyhow::Result<()> {
  let origin = Url::parse(raw).map_err(|_| anyhow::anyhow!("invalid hosted APP_BASE_URL"))?;
  ensure!(
    origin.scheme() == "https" && origin.origin().ascii_serialization() == raw,
    "the hosted APP_BASE_URL must be a canonical HTTPS origin"
  );
  Ok(())
}

fn strong_session_secret(secret: &str) -> anyhow::Result<()> {
  let mut key = [0u8; 32];
  ensure!(
    hex::decode_to_slice(secret, &mut key).is_ok() && key.iter().any(|byte| *byte != key[0]),
    "set SESSION_SECRET to a generated 32-byte hex key for the hosted gateway"
  );
  Ok(())
}

pub(crate) async fn protect_session_origin(
  State(console_origin): State<HeaderValue>,
  request: Request,
  next: Next,
) -> Result<Response, ApiError> {
  if !request.method().is_safe() && cookie::read_session_token(request.headers()).is_some() {
    let mut origins = request.headers().get_all(ORIGIN).iter();
    if origins.next() != Some(&console_origin) || origins.next().is_some() {
      return Err(ApiError::Forbidden("session writes require the configured console origin".into()));
    }
  }
  Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hosted_database_requires_verified_tls_over_tcp() {
    assert!(db::verify_hosted_url("postgres://database.example/tokencraft?sslmode=verify-full").is_ok());
    for mode in ["disable", "allow", "prefer", "require", "verify-ca"] {
      assert!(db::verify_hosted_url(&format!("postgres://database.example/tokencraft?sslmode={mode}")).is_err());
    }
    assert!(db::verify_hosted_url("postgres://database.example/tokencraft?host=%2Ftmp&sslmode=verify-full").is_err());
  }

  #[test]
  fn hosted_console_origin_and_session_secret_are_validated() {
    assert!(verify_origin("https://console.example:8443").is_ok());
    for invalid in ["http://console.example", "https://user@console.example", "https://console.example/app",
      "https://console.example?x=y", "https://console.example#foo", "https://console.example:443"] {
      assert!(verify_origin(invalid).is_err(), "{invalid}");
    }
    assert!(strong_session_secret("dev-insecure-session-secret-change-me").is_err());
    assert!(strong_session_secret(&"aa".repeat(32)).is_err());
    assert!(strong_session_secret(&"ab12".repeat(16)).is_ok());
  }
}
