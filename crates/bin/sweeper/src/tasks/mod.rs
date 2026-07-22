//! Sweeper tasks — one module per task. Each scans the database for work it owns and
//! takes care of it; the runner makes every task infallible (report + backoff, never die).

mod api_key_last_used_update;
mod gravatar_fetch_task;

pub use api_key_last_used_update::ApiKeyLastUsedUpdate;
pub use gravatar_fetch_task::GravatarFetchTask;

use std::time::Duration;

use bucket_client::BucketClient;

/// Shared handles every task gets: the pool, an HTTP client, and (optionally) object
/// storage. Cheap to clone.
#[derive(Clone)]
pub struct TaskContext {
  pub pool: db::PgPool,
  pub http: reqwest::Client,
  pub bucket: Option<BucketClient>,
}

/// One background task. `run_once` performs a single bounded pass and reports how many
/// items it handled (0 = idle); the runner owns retries, backoff, and shutdown.
pub trait SweeperTask: Send + Sync + 'static {
  fn name(&self) -> &'static str;
  /// Wait between passes when a pass found nothing to do.
  fn idle_wait(&self) -> Duration {
    Duration::from_secs(15)
  }
  /// Base backoff after a failed pass (doubles per consecutive error, capped by the runner).
  fn error_backoff(&self) -> Duration {
    Duration::from_secs(30)
  }
  fn run_once(&self, ctx: &TaskContext) -> impl std::future::Future<Output = anyhow::Result<usize>> + Send;
}
