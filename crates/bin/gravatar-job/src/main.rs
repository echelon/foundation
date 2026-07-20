//! gravatar-job — a standalone background worker that fills in gravatar avatars.
//!
//! It scans `user_avatars` for rows that still need a gravatar existence probe
//! (`gravatar_checked_at IS NULL` — set on signup and reset on every email change), probes
//! `gravatar.com` for each, and records the result. Deliberately simple: ONE main loop, batch of
//! work, graceful failure/backoff, sleep-and-retry when idle — not the full `queue_tasks`
//! dispatcher (that's for provider jobs).
//!
//! Existence probe: `GET https://gravatar.com/avatar/{sha256(trim+lower(email))}?s=512&d=404`.
//! `d=404` makes gravatar answer 404 when the address has no avatar (200 ⇒ one exists). No API key
//! needed. NOTE: byte-mirroring the image into `user_avatar_files` (accounts.md §2) needs object
//! storage that isn't wired yet — until then we record existence and promote the avatar `source` to
//! `gravatar` so it's observable end-to-end.

use std::time::Duration;

use db_queries::user_avatars;

/// How many avatars to probe per loop iteration.
const BATCH_SIZE: i64 = 25;
/// Idle wait when there's no work (or after a DB error) before scanning again.
const IDLE_WAIT: Duration = Duration::from_secs(15);
/// Backoff after a probe/transport error spike, to avoid hammering a flaky network.
const ERROR_BACKOFF: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // Env files first (convention: crates/bin/gravatar-job/config/gravatar-job.{env}.env), so
  // `cargo run -p gravatar-job` needs no hand-supplied env vars and RUST_LOG from a file works.
  let loaded = config::load_env_files("gravatar-job");

  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,gravatar_job=debug".into()),
    )
    .init();

  for path in &loaded {
    tracing::info!("loaded env file: {}", path.display());
  }

  let cfg = config::Config::load()?;
  let pool = db::connect(&cfg.database_url).await?;
  tracing::info!("gravatar-job connected to database; starting poll loop");

  let http = client_core::http_client("foundation/gravatar-job");

  loop {
    // ctrl-c wins against whatever we're doing, so shutdown is prompt even under a backlog.
    tokio::select! {
      _ = tokio::signal::ctrl_c() => {
        tracing::info!("shutdown signal received — exiting");
        break;
      }
      outcome = run_batch(&pool, &http) => {
        match outcome {
          // No work — wait before scanning again.
          Ok(0) => if wait_or_shutdown(IDLE_WAIT).await { break; },
          Ok(n) => tracing::info!(processed = n, "processed a batch"), // loop straight back for more
          Err(e) => {
            tracing::error!(error = %e, "batch failed — backing off");
            if wait_or_shutdown(ERROR_BACKOFF).await {
              break;
            }
          }
        }
      }
    }
  }

  Ok(())
}

/// Sleep for `dur`, returning `true` early if a shutdown (ctrl-c) is requested meanwhile.
async fn wait_or_shutdown(dur: Duration) -> bool {
  tokio::select! {
    _ = tokio::time::sleep(dur) => false,
    _ = tokio::signal::ctrl_c() => true,
  }
}

/// Probe one batch of unpolled avatars. Returns how many were processed (0 ⇒ idle). A single row's
/// failure is isolated: on a probe/transport error we leave the row unpolled (so it's retried next
/// pass) and move on — the batch only errors on a DB-level failure fetching the work.
async fn run_batch(pool: &db::PgPool, http: &reqwest::Client) -> anyhow::Result<usize> {
  let batch = user_avatars::list_unpolled_avatars(pool, BATCH_SIZE).await?;
  if batch.is_empty() {
    return Ok(0);
  }

  let mut processed = 0;
  for avatar in batch {
    let Some(hash) = avatar.gravatar_hash.as_deref() else {
      // No hash to probe with — mark checked (not found) so it leaves the queue.
      let _ = user_avatars::mark_gravatar_checked(pool, &avatar.user_id, false).await;
      processed += 1;
      continue;
    };

    match probe_gravatar(http, hash).await {
      Ok(found) => {
        if let Err(e) = user_avatars::mark_gravatar_checked(pool, &avatar.user_id, found).await {
          tracing::error!(error = %e, user_id = %avatar.user_id, "failed to record probe result");
        } else {
          tracing::debug!(user_id = %avatar.user_id, found, "probed gravatar");
          processed += 1;
        }
      }
      Err(e) => {
        // Transport error — leave the row unpolled so a later pass retries it (graceful failure).
        tracing::warn!(error = %e, user_id = %avatar.user_id, "gravatar probe failed — will retry");
      }
    }
  }
  Ok(processed)
}

/// `true` if a gravatar exists for this hash. Uses `d=404` so a missing avatar returns 404.
async fn probe_gravatar(http: &reqwest::Client, hash: &str) -> reqwest::Result<bool> {
  let url = format!("https://gravatar.com/avatar/{hash}?s=512&d=404");
  let resp = http.get(&url).send().await?;
  Ok(resp.status().is_success())
}
