//! google-gemini-outbound-job — the OUTBOUND worker for `provider = 'google'` jobs.
//!
//! Some Google models (Nano Banana) are fully SYNCHRONOUS at the provider, so
//! they must never run inside the api-gateway web server: this binary claims
//! queued google jobs from the DB (in order, `FOR UPDATE SKIP LOCKED` leases)
//! and runs them on a bounded pool (~20 concurrent). Veo video jobs are async:
//! this worker only STARTS their operations; google-gemini-polling-job lands
//! them. Scale = more pods; the claim query already partitions safely.
//!
//! Graceful pre-kill: on SIGTERM/ctrl-c the worker STOPS claiming, drains the
//! in-flight jobs to completion, then exits.

mod worker;

use bucket_client::BucketClientBuilder;
use google_gemini_api_client::creds::google_gemini_api_key::GoogleGeminiApiKey;
use google_gemini_dispatch::context::DispatchContext;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let loaded = config::load_env_files("google-gemini-outbound-job");

  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,google_gemini_outbound_job=debug".into()),
    )
    .init();

  for path in &loaded {
    tracing::info!("loaded env file: {}", path.display());
  }

  let cfg = config::Config::load()?;
  let pool = db::connect(&cfg.database_url).await?;

  let Some(gemini_key) = cfg.providers.gemini.clone() else {
    anyhow::bail!("GEMINI_API_KEY is required — this worker exists to talk to Google");
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
      tracing::warn!("R2 storage not configured — image jobs will fail at ingest");
      None
    }
  };

  let ctx = DispatchContext {
    pool,
    api_key: GoogleGeminiApiKey::new(gemini_key),
    bucket,
    http: client_core::http_client("foundation/google-gemini-outbound-job"),
  };

  let (shutdown_tx, shutdown_rx) = watch::channel(false);
  tracing::info!("google-gemini-outbound-job connected; claiming provider='google' jobs");
  let worker = tokio::spawn(worker::run(ctx, shutdown_rx));

  shutdown_signal().await;
  tracing::info!("shutdown signal received — draining in-flight jobs");
  let _ = shutdown_tx.send(true);
  let _ = worker.await;
  tracing::info!("drained; exiting");
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
