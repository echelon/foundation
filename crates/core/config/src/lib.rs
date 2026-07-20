//! Environment / config loading (environments.md). Reads `.env` in dev, then the
//! process environment. Provider credential pools are optional — a missing key means
//! that provider is simply unroutable, not a boot failure.

use anyhow::{Context, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Environment {
    Dev,
    Stage,
    Prod,
    Test,
}

impl Environment {
    fn parse(s: &str) -> Environment {
        match s {
            "prod" | "production" => Environment::Prod,
            "stage" | "staging" => Environment::Stage,
            "test" => Environment::Test,
            _ => Environment::Dev,
        }
    }
    pub fn is_dev(&self) -> bool {
        matches!(self, Environment::Dev)
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub environment: Environment,
    pub database_url: String,
    pub api_gateway_port: u16,
    pub cdn_base_url: String,
    /// HMAC secret for signing session JWTs (auth.md §1). Rotate in prod.
    pub session_secret: String,
    /// Cookie `Domain` attribute — `.example.com` in prod so it spans the apex (auth.md §1).
    pub cookie_domain: String,
    /// Whether to set the `Secure` cookie attribute (off for plain-http local dev).
    pub cookie_secure: bool,
    /// Public base URL of the frontend app, used to build emailed links (e.g. the password-reset
    /// link `{app_base_url}/reset-password?token=…`).
    pub app_base_url: String,
    pub email: EmailConfig,
    pub providers: ProviderKeys,
}

/// Email-sending config (email.md §1). A missing `resend_api_key` means email is unconfigured —
/// flows that would send just log and no-op rather than failing the request.
#[derive(Clone, Debug, Default)]
pub struct EmailConfig {
    pub resend_api_key: Option<String>,
    /// Sender in display form, e.g. `"Foundation <noreply@example.com>"`. Must be a Resend-verified
    /// sender domain.
    pub from_address: String,
}

#[derive(Clone, Debug, Default)]
pub struct ProviderKeys {
    pub kinovi: Option<String>,
    pub openai: Option<String>,
    pub gemini: Option<String>,
}

fn opt(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

impl Config {
    /// Load config, sourcing `.env` first (best-effort — absent file is fine).
    pub fn load() -> Result<Config> {
        let _ = dotenvy::dotenv();
        Ok(Config {
            environment: Environment::parse(&std::env::var("FOUNDATION_ENV").unwrap_or_default()),
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            api_gateway_port: opt("API_GATEWAY_PORT")
                .and_then(|p| p.parse().ok())
                .unwrap_or(4000),
            cdn_base_url: std::env::var("CDN_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:9000/foundation".into()),
            session_secret: std::env::var("SESSION_SECRET")
                .unwrap_or_else(|_| "dev-insecure-session-secret-change-me".into()),
            cookie_domain: std::env::var("COOKIE_DOMAIN").unwrap_or_else(|_| "localhost".into()),
            cookie_secure: opt("COOKIE_SECURE").map(|v| v == "true").unwrap_or(false),
            app_base_url: std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
            email: EmailConfig {
                resend_api_key: opt("RESEND_API_KEY"),
                from_address: std::env::var("EMAIL_FROM_ADDRESS")
                    .unwrap_or_else(|_| "Foundation <noreply@localhost>".into()),
            },
            providers: ProviderKeys {
                kinovi: opt("KINOVI_API_KEY"),
                openai: opt("OPENAI_API_KEY"),
                gemini: opt("GEMINI_API_KEY"),
            },
        })
    }
}
