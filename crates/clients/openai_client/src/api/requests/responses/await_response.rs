//! The poll loop: call `get_response` on an interval until the response reaches a terminal
//! state (or we give up). Chosen over webhooks deliberately — polling needs no inbound
//! endpoint and matches the create→poll shape of our other providers (dispatch.md).

use std::time::Duration;

use crate::api::requests::responses::get_response::{get_response, GetResponseArgs, ResponseSnapshot};
use crate::api::types::response_types::response_id::ResponseId;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::openai_client_error::OpenAiClientError;
use crate::error::openai_error::OpenAiError;

#[derive(Debug)]
pub struct AwaitResponseArgs<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub response_id: &'a ResponseId,
}

/// Poll pacing. Defaults: every 2s, give up after 10 minutes (image generations typically
/// land well inside a minute; LLM turns in seconds).
#[derive(Clone, Copy, Debug)]
pub struct PollConfig {
  pub interval: Duration,
  pub timeout: Duration,
}

impl Default for PollConfig {
  fn default() -> Self {
    Self { interval: Duration::from_secs(2), timeout: Duration::from_secs(600) }
  }
}

/// Poll until terminal. Returns the terminal snapshot (which may be `Failed` /
/// `Incomplete` — inspect `status`/`failure`; only transport-level problems are `Err`).
pub async fn await_response(
  args: AwaitResponseArgs<'_>,
  config: PollConfig,
) -> Result<ResponseSnapshot, OpenAiError> {
  let started = std::time::Instant::now();
  loop {
    let snapshot = get_response(GetResponseArgs { api_key: args.api_key, response_id: args.response_id }).await?;
    if snapshot.status.is_terminal() {
      return Ok(snapshot);
    }
    if started.elapsed() + config.interval > config.timeout {
      return Err(OpenAiClientError::PollTimeout { waited: started.elapsed() }.into());
    }
    tokio::time::sleep(config.interval).await;
  }
}
