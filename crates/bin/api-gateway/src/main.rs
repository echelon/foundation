//! api-gateway binary — thin: load env files (by convention), init tracing, hand off to the
//! library. `cargo run -p api-gateway` needs no hand-supplied env vars.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // Env files first, so RUST_LOG from a file shapes the tracing filter below.
  let loaded = config::load_env_files("api-gateway");

  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,api_gateway=debug".into()),
    )
    .init();

  for path in &loaded {
    tracing::info!("loaded env file: {}", path.display());
  }

  let cfg = config::Config::load()?;
  api_gateway::run(cfg).await
}
