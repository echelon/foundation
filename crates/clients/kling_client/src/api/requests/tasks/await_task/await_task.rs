use std::time::Duration;

use crate::api::requests::tasks::get_tasks::{get_tasks, GetTasksArgs, TaskQuery};
use crate::api::types::task_types::kling_task::KlingTask;
use crate::api::types::task_types::kling_task_id::KlingTaskId;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;
use crate::error::kling_specific_api_error::KlingSpecificApiError;

/// Poll cadence for [`await_task`]. Kling video generations run for minutes,
/// so the default is a relaxed 5s interval with a 30-minute ceiling.
#[derive(Clone, Copy, Debug)]
pub struct PollConfig {
  pub interval: Duration,
  pub timeout: Duration,
}

impl Default for PollConfig {
  fn default() -> Self {
    Self { interval: Duration::from_secs(5), timeout: Duration::from_secs(1800) }
  }
}

#[derive(Clone, Debug)]
pub struct AwaitTaskArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub task_id: &'a KlingTaskId,
}

/// Poll `GET /tasks` until the task reaches a terminal status, then return it.
///
/// A failed generation is an `Ok(KlingTask)` with `status == Failed` — only
/// transport problems and the poll timeout are `Err`. When a `callback_url`
/// was set, prefer reacting to the callback and confirming with one poll
/// instead of looping here.
pub async fn await_task(args: AwaitTaskArgs<'_>, config: PollConfig) -> Result<KlingTask, KlingError> {
  let started = std::time::Instant::now();
  loop {
    let tasks = get_tasks(GetTasksArgs {
      api_key: args.api_key,
      query: TaskQuery::ByTaskIds(vec![args.task_id.clone()]),
    })
    .await?;

    let task = tasks
      .into_iter()
      .find(|t| &t.id == args.task_id)
      .ok_or(KlingSpecificApiError::NotFound { raw_http_body: None })?;

    if task.status.is_terminal() {
      return Ok(task);
    }
    if started.elapsed() + config.interval > config.timeout {
      return Err(KlingClientError::PollTimeout { waited: started.elapsed() }.into());
    }
    tokio::time::sleep(config.interval).await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_poll_config_is_sane_for_multi_minute_generations() {
    let config = PollConfig::default();
    assert_eq!(config.interval, Duration::from_secs(5));
    assert_eq!(config.timeout, Duration::from_secs(1800));
  }
}
