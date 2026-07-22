//! The infallible task loop. Every sweeper task runs inside [`run_task`]: one batch of
//! work per pass, errors REPORTED and retried with exponential backoff (never fatal to
//! the task, never fatal to the process), prompt shutdown via the shared watch channel.

use std::time::Duration;

use tokio::sync::watch;

use crate::tasks::{SweeperTask, TaskContext};

/// Consecutive-error backoff: base × 2^errors, capped.
fn backoff(base: Duration, consecutive_errors: u32) -> Duration {
  base.saturating_mul(2u32.saturating_pow(consecutive_errors.min(5)))
}

/// Drive one task forever (until shutdown). A pass that finds work loops straight back
/// for more; an idle pass waits `idle_wait`; an error logs + backs off and the loop
/// CONTINUES — a sweeper task is infallible by construction.
pub async fn run_task<T: SweeperTask>(task: T, ctx: TaskContext, mut shutdown: watch::Receiver<bool>) {
  let name = task.name();
  tracing::info!(task = name, "task started");
  let mut consecutive_errors: u32 = 0;
  loop {
    let wait = tokio::select! {
      _ = shutdown.changed() => break,
      outcome = task.run_once(&ctx) => match outcome {
        Ok(0) => {
          consecutive_errors = 0;
          task.idle_wait()
        }
        Ok(n) => {
          consecutive_errors = 0;
          tracing::info!(task = name, processed = n, "processed a batch");
          Duration::ZERO // more work may be waiting — go straight back
        }
        Err(e) => {
          consecutive_errors += 1;
          let wait = backoff(task.error_backoff(), consecutive_errors - 1);
          tracing::error!(task = name, error = %e, consecutive_errors, backoff_secs = wait.as_secs(), "pass failed — backing off");
          wait
        }
      },
    };
    if !wait.is_zero() {
      tokio::select! {
        _ = shutdown.changed() => break,
        _ = tokio::time::sleep(wait) => {}
      }
    }
  }
  tracing::info!(task = name, "task stopped");
}
