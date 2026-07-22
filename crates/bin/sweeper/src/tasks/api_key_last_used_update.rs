//! Out-of-band `api_keys.last_used_at` freshness. The gateway's hot auth path is
//! read-only (it stamps inline at most once per 10 minutes per key); this task closes
//! the gap by deriving each key's true latest use from the jobs it created and advancing
//! any stamp that lags. One UPDATE per pass, however many keys need it.

use std::time::Duration;

use super::{SweeperTask, TaskContext};

pub struct ApiKeyLastUsedUpdate;

impl SweeperTask for ApiKeyLastUsedUpdate {
  fn name(&self) -> &'static str {
    "api_key_last_used_update"
  }

  /// Freshness sweep, not a queue drain — a minute of lag is fine.
  fn idle_wait(&self) -> Duration {
    Duration::from_secs(60)
  }

  async fn run_once(&self, ctx: &TaskContext) -> anyhow::Result<usize> {
    let advanced = db_queries::api_keys::refresh_api_keys_last_used_from_jobs(&ctx.pool).await?;
    // One sweep IS the whole pass — report progress but never spin (idle-wait either way).
    if advanced > 0 {
      tracing::debug!(advanced, "advanced stale api_key last_used_at stamps");
    }
    Ok(0)
  }
}
