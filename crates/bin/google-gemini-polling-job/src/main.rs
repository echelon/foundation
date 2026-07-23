//! google-gemini-polling-job — drives STARTED Google video operations (Veo) to
//! completion: polls incomplete `provider = 'google'` jobs/slices, downloads
//! finished videos, ingests them to R2, and lands the records (partial
//! completion per slice). Image jobs never appear here — Nano Banana completes
//! synchronously inside google-gemini-outbound-job.
//!
//! Infallible by construction: poll errors are logged and retried next tick.
//! SIGTERM/ctrl-c stops promptly (polling holds no long-lived provider work).

mod poller;

use bucket_client::BucketClientBuilder;
use google_gemini_api_client::creds::google_gemini_api_key::GoogleGeminiApiKey;
use google_gemini_dispatch::context::DispatchContext;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let loaded = config::load_env_files("google-gemini-polling-job");

  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,google_gemini_polling_job=debug".into()),
    )
    .init();

  for path in &loaded {
    tracing::info!("loaded env file: {}", path.display());
  }

  let cfg = config::Config::load()?;
  let pool = db::connect(&cfg.database_url).await?;

  let Some(gemini_key) = cfg.providers.gemini.clone() else {
    anyhow::bail!("GEMINI_API_KEY is required — this worker exists to poll Google");
  };
  let bucket = match &cfg.storage {
    Some(s) => Some(
      BucketClientBuilder::new()
        .endpoint(&s.endpoint)
        .access_key(&s.access_key_id)
        .secret_key(&s.secret_access_key)
        .region_name(&s.region)
        .bucket_name(&s.bucket_name)
        .build()
        .map_err(|e| anyhow::anyhow!("bucket client: {e}"))?,
    ),
    None => {
      tracing::warn!("R2 storage not configured — completed videos will fail at ingest");
      None
    }
  };

  let ctx = DispatchContext {
    pool,
    api_key: GoogleGeminiApiKey::new(gemini_key),
    bucket,
    http: client_core::http_client("foundation/google-gemini-polling-job"),
  };

  let (shutdown_tx, shutdown_rx) = watch::channel(false);
  tracing::info!("google-gemini-polling-job connected; polling provider='google' operations");
  let poller = tokio::spawn(poller::run(ctx, shutdown_rx));

  shutdown_signal().await;
  tracing::info!("shutdown signal received — stopping");
  let _ = shutdown_tx.send(true);
  let _ = poller.await;
  Ok(())
}

/// SIGTERM (k8s pre-kill) or ctrl-c (local), whichever lands first.
async fn shutdown_signal() {
  let ctrl_c = tokio::signal::ctrl_c();
  #[cfg(unix)]
  {
    let mut sigterm =
      tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
      _ = ctrl_c => {}
      _ = sigterm.recv() => {}
    }
  }
  #[cfg(not(unix))]
  {
    let _ = ctrl_c.await;
  }
}
