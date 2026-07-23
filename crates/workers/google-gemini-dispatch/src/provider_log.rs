//! Outbound provider-request logging (logging.md §1 point 3): the
//! `provider_calls` row is written BEFORE the send — its `id` is the
//! request_id, so even a failed send leaves the exact payload behind.
//! Worker twin of the gateway's `generation/provider_log.rs`.

use db_queries::provider_calls::{self, NewProviderCall, ProviderCallResult};
use db_queries::{JobSlice, ProviderCall};
use foundation_router::image::route::ImageRoute;
use foundation_router::video::route::VideoRoute;
use ids::JobId;
use serde_json::json;
use sqlx::PgPool;

/// Which route the call belongs to — one logging path for both media types.
pub enum RouteRef<'a> {
  Image(&'a ImageRoute),
  Video(&'a VideoRoute),
}

impl RouteRef<'_> {
  fn request_json(&self, index: usize) -> Result<serde_json::Value, foundation_router::error::RouterError> {
    match self {
      Self::Image(r) => r.request_json(index),
      Self::Video(r) => r.request_json(index),
    }
  }

  fn provider(&self) -> catalog::Provider {
    match self {
      Self::Image(r) => r.provider(),
      Self::Video(r) => r.provider(),
    }
  }

  fn endpoint_url(&self) -> String {
    match self {
      Self::Image(r) => r.endpoint_url(),
      Self::Video(r) => r.endpoint_url(),
    }
  }
}

pub struct LogOutboundRequestArgs<'a> {
  pub job_id: &'a JobId,
  /// The fan-out slice this call belongs to; None for single-call jobs.
  pub slice: Option<&'a JobSlice>,
  pub route: RouteRef<'a>,
  /// Which slice of the route to serialize (0 for single-call jobs).
  pub index: usize,
}

/// Write the pre-send `provider_calls` row. Never blocks the dispatch: a
/// serialization failure logs and yields `None`.
pub async fn log_outbound_request(pool: &PgPool, args: LogOutboundRequestArgs<'_>) -> sqlx::Result<Option<ProviderCall>> {
  let request_body = match args.route.request_json(args.index) {
    Ok(json) => json,
    Err(e) => {
      tracing::error!("could not serialize outbound request for {} slice {}: {e}", args.job_id, args.index);
      return Ok(None);
    }
  };
  let call = provider_calls::create_provider_call(
    pool,
    NewProviderCall {
      job_id: args.job_id,
      job_slice_id: args.slice.map(|s| &s.id),
      provider: args.route.provider().as_str(),
      url: &args.route.endpoint_url(),
      request_body: &request_body,
      is_dry_run: false,
    },
  )
  .await?;
  Ok(Some(call))
}

/// Finish the pre-send row with what came back (a provider handle / output
/// count) or the send error. Logging must never fail the dispatch itself.
pub async fn record_call_result(
  pool: &PgPool,
  call: Option<&ProviderCall>,
  outcome: Result<&serde_json::Value, &str>,
  sent_at: std::time::Instant,
) -> sqlx::Result<()> {
  let Some(call) = call else { return Ok(()) };
  let (provider_job_id, response_body) = match outcome {
    Ok(body) => (body.get("id").and_then(|v| v.as_str()).map(str::to_string), body.clone()),
    Err(e) => (None, json!({ "error": e })),
  };
  provider_calls::set_provider_call_result(
    pool,
    ProviderCallResult {
      id: &call.id,
      provider_job_id: provider_job_id.as_deref(),
      response_body: Some(&response_body),
      latency_ms: Some(sent_at.elapsed().as_millis().min(i32::MAX as u128) as i32),
    },
  )
  .await
}
