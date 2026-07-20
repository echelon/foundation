//! Environment / config loading (environments.md).
//!
//! Every service reads its environment from **convention-named env files** loaded at startup by
//! [`load_env_files`] — `cargo run -p {service}` needs no hand-supplied env vars. Per service
//! (`{app}` = the binary name, files live in `crates/bin/{app}/config/`):
//!
//!   - `{app}.common.env`              — all environments (committed)
//!   - `{app}.development.env`         — dev defaults (committed; NO real secrets)
//!   - `{app}.development-secrets.env` — dev secrets (**gitignored**; see the `.example` template)
//!   - `{app}.production.env`          — prod config (committed; real secrets injected at deploy)
//!
//! `FOUNDATION_ENV` (read from the process environment BEFORE any file) picks the set; default is
//! dev. Real process env vars always win over file values, and more-specific files win over
//! `common` — so a deploy platform or a one-off `DATABASE_URL=… cargo run` override just works.
//! Provider credential pools are optional — a missing key means that provider is simply
//! unroutable, not a boot failure.

use std::path::{Path, PathBuf};

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

/// Load the convention-named env files for `app_name` (see the crate docs for the file set).
/// Call this FIRST in every binary's `main`, before initializing logging (so `RUST_LOG` from a
/// file takes effect) and before [`Config::load`].
///
/// Semantics:
///   - `FOUNDATION_ENV` is read from the process environment only (a file can't switch it —
///     chicken-and-egg), defaulting to dev.
///   - Files load most-specific-first (`…-secrets` → `…{env}` → `common`); dotenvy never
///     overrides an already-set var, so earlier-loaded files and the real process env win.
///   - Each file is searched across the service dir (`crates/bin/{app}/config`), `./config`,
///     and `.` — so running from the repo root or the crate dir both work.
///   - Missing files are fine (the secrets file won't exist until someone creates it).
///
/// Returns the paths actually loaded so the caller can log them once tracing is up.
pub fn load_env_files(app_name: &str) -> Vec<PathBuf> {
    let environment = Environment::parse(&std::env::var("FOUNDATION_ENV").unwrap_or_default());

    let file_names: Vec<String> = match environment {
        Environment::Dev => vec![
            format!("{app_name}.development-secrets.env"), // gitignored
            format!("{app_name}.development.env"),
            format!("{app_name}.common.env"),
        ],
        Environment::Test => vec![format!("{app_name}.test.env"), format!("{app_name}.common.env")],
        Environment::Stage => vec![format!("{app_name}.stage.env"), format!("{app_name}.common.env")],
        Environment::Prod => vec![format!("{app_name}.production.env"), format!("{app_name}.common.env")],
    };

    let service_config_dir = format!("crates/bin/{app_name}/config");
    let search_dirs = [service_config_dir.as_str(), "config", "."];

    let mut loaded = Vec::new();
    for file_name in &file_names {
        for dir in &search_dirs {
            let path = Path::new(dir).join(file_name);
            if path.is_file() {
                // from_path never overrides vars that are already set.
                if dotenvy::from_path(&path).is_ok() {
                    loaded.push(path);
                }
                break; // first hit wins; don't load the same file name from a later dir
            }
        }
    }
    loaded
}

impl Config {
    /// Parse config from the process environment. Env files are NOT read here — call
    /// [`load_env_files`] first (every binary's `main` does).
    pub fn load() -> Result<Config> {
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
