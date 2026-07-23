//! The poll loop: list incomplete google jobs/slices, advance each one.
//! Veo renders take minutes — a relaxed tick is plenty.

use std::time::Duration;

use tokio::sync::watch;

use google_gemini_dispatch::context::DispatchContext;
use google_gemini_dispatch::video_poll_flow::{poll_and_finish_unsliced_video_job, poll_and_finish_video_slice};

const TICK: Duration = Duration::from_secs(5);
const POLL_BATCH: i64 = 50;

pub async fn run(ctx: DispatchContext, mut shutdown: watch::Receiver<bool>) {
  let mut ticker = tokio::time::interval(TICK);
  ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
  loop {
    tokio::select! {
      _ = shutdown.changed() => break,
      _ = ticker.tick() => {}
    }
    if let Err(e) = poll_pass(&ctx).await {
      tracing::error!("poll pass errored: {e}");
    }
  }
  tracing::info!("poller stopped");
}

async fn poll_pass(ctx: &DispatchContext) -> sqlx::Result<()> {
  for job in db_queries::jobs::list_pollable_unsliced_jobs(&ctx.pool, "google", POLL_BATCH).await? {
    // Only video jobs ever wait on a provider operation (belt-and-suspenders:
    // image jobs complete synchronously in the outbound worker).
    if job.media_type != catalog::MediaType::Video.as_str() {
      continue;
    }
    if let Err(e) = poll_and_finish_unsliced_video_job(ctx, &job).await {
      tracing::warn!("poll of {} failed (will retry): {e}", job.id);
    }
  }
  for slice in db_queries::job_slices::list_pollable_slices(&ctx.pool, "google", POLL_BATCH).await? {
    if let Err(e) = poll_and_finish_video_slice(ctx, &slice).await {
      tracing::warn!("poll of slice {} failed (will retry): {e}", slice.id);
    }
  }
  Ok(())
}
