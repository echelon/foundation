//! sweeper — the out-of-band background worker. Runs several independent tasks (one
//! module each, `src/tasks/`) on the multithreaded runtime; each task scans the database
//! for work it owns and takes care of it. Tasks are INFALLIBLE by construction: the
//! runner (`src/runner.rs`) turns every error into a report + exponential backoff and
//! keeps going. Ctrl-C/SIGTERM stops all tasks promptly.
//!
//! Current tasks:
//!   - gravatar_fetch            probe + mirror gravatars (was the `gravatar-job` binary)
//!   - api_key_last_used_update  advance stale api_keys.last_used_at from jobs activity

mod runner;
mod tasks;

use bucket_client::BucketClientBuilder;
use tokio::sync::watch;

use tasks::{ApiKeyLastUsedUpdate, GravatarFetchTask, TaskContext};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // Env files first (convention: crates/bin/sweeper/config/sweeper.{env}.env), so
  // `cargo run -p sweeper` needs no hand-supplied env vars and RUST_LOG from a file works.
  let loaded = config::load_env_files("sweeper");

  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sweeper=debug".into()),
    )
    .init();

  for path in &loaded {
    tracing::info!("loaded env file: {}", path.display());
  }

  let cfg = config::Config::load()?;
  let pool = db::connect(&cfg.database_url).await?;

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
      tracing::info!("Object storage not configured — Gravatar imports are disabled");
      None
    }
  };

  let ctx = TaskContext { pool, http: client_core::http_client("foundation/sweeper"), bucket };
  let (shutdown_tx, shutdown_rx) = watch::channel(false);

  tracing::info!(mirroring = ctx.bucket.is_some(), "sweeper connected to database; starting tasks");
  let handles = vec![
    tokio::spawn(runner::run_task(GravatarFetchTask, ctx.clone(), shutdown_rx.clone())),
    tokio::spawn(runner::run_task(ApiKeyLastUsedUpdate, ctx.clone(), shutdown_rx.clone())),
  ];

  tokio::signal::ctrl_c().await?;
  tracing::info!("shutdown signal received — stopping tasks");
  let _ = shutdown_tx.send(true);
  for handle in handles {
    let _ = handle.await;
  }
  tracing::info!("all tasks stopped");
  Ok(())
}
