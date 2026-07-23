//! The claim-and-run loop: lease due google jobs, run each on the bounded task
//! pool (image jobs synchronously to completion, video jobs start-only), and
//! drain gracefully on shutdown.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;

use google_gemini_dispatch::context::DispatchContext;
use google_gemini_dispatch::image_dispatch::dispatch_image_job;
use google_gemini_dispatch::video_dispatch::dispatch_video_job;

const TICK: Duration = Duration::from_secs(2);
/// Bounded concurrency — one synchronous Nano Banana call can hold a slot for
/// tens of seconds; more throughput = more pods, not a bigger pool.
const CONCURRENCY: usize = 20;
/// Claim lease — generous because sync image fan-outs run inside it; a worker
/// that dies mid-run surrenders its claims after this long.
const DISPATCH_LEASE_SECONDS: f64 = 600.0;

pub async fn run(ctx: DispatchContext, mut shutdown: watch::Receiver<bool>) {
  let semaphore = Arc::new(Semaphore::new(CONCURRENCY));
  let mut in_flight: JoinSet<()> = JoinSet::new();
  let mut ticker = tokio::time::interval(TICK);
  ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

  loop {
    tokio::select! {
      _ = shutdown.changed() => break,
      _ = ticker.tick() => {}
    }
    // Reap finished tasks without blocking.
    while in_flight.try_join_next().is_some() {}

    // Claim only what we have free slots for — leased jobs we can't start yet
    // would just sit on our lease for no reason.
    let free = semaphore.available_permits();
    if free == 0 {
      continue;
    }
    let claimed =
      match db_queries::jobs::claim_due_dispatch_jobs(&ctx.pool, "google", DISPATCH_LEASE_SECONDS, free as i64)
        .await
      {
        Ok(jobs) => jobs,
        Err(e) => {
          tracing::error!("claim failed: {e}");
          continue;
        }
      };
    for job in claimed {
      let permit = semaphore.clone().acquire_owned().await.expect("semaphore never closes");
      let ctx = ctx.clone();
      in_flight.spawn(async move {
        let _permit = permit;
        let result = if job.media_type == catalog::MediaType::Video.as_str() {
          dispatch_video_job(&ctx, &job).await
        } else {
          dispatch_image_job(&ctx, &job).await
        };
        if let Err(e) = result {
          tracing::error!("dispatch of {} errored: {e}", job.id);
        }
      });
    }
  }

  // Drain: no new claims (loop exited); wait out the live ones.
  tracing::info!(in_flight = in_flight.len(), "draining");
  while in_flight.join_next().await.is_some() {}
}
