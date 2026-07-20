//! api-gateway binary — thin: load config, init tracing, hand off to the library.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,api_gateway=debug".into()),
    )
    .init();

  let cfg = config::Config::load()?;
  api_gateway::run(cfg).await
}
