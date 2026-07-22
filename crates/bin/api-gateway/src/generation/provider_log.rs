//! Outbound provider-request logging (logging.md §1 point 3): the `provider_calls` row is
//! written BEFORE the send — its `id` is the request_id, minted up front so even a failed
//! or never-made (dry-run) send leaves the exact payload behind for debug aggregation.

use db_queries::provider_calls::{self, NewProviderCall};
use db_queries::{JobSlice, ProviderCall};
use foundation_router::image::route::ImageRoute;
use ids::JobId;
use sqlx::PgExecutor;

/// Everything one outbound image call is about (named fields — jobs.md args-struct rule).
pub struct LogOutboundImageRequestArgs<'a> {
  pub job_id: &'a JobId,
  /// The fan-out slice this call belongs to; None for single-call jobs.
  pub slice: Option<&'a JobSlice>,
  pub route: &'a ImageRoute,
  /// Which slice of the route to serialize (0 for single-call jobs).
  pub index: usize,
  /// Dry run: the payload is logged but the request will never be sent.
  pub is_dry_run: bool,
}

/// Serialize the exact provider-native payload and write the pre-send `provider_calls`
/// row. Never blocks the caller: a serialization failure is logged and yields `None`
/// (the dispatch itself may still proceed — the send path serializes independently).
pub async fn log_outbound_image_request<'e, E: PgExecutor<'e>>(
  exec: E,
  args: LogOutboundImageRequestArgs<'_>,
) -> sqlx::Result<Option<ProviderCall>> {
  let request_body = match args.route.request_json(args.index) {
    Ok(json) => json,
    Err(e) => {
      tracing::error!("could not serialize outbound request for {} slice {}: {e}", args.job_id, args.index);
      return Ok(None);
    }
  };
  let call = provider_calls::create_provider_call(
    exec,
    NewProviderCall {
      job_id: args.job_id,
      job_slice_id: args.slice.map(|s| &s.id),
      provider: "openai",
      url: args.route.endpoint_url(),
      request_body: &request_body,
      is_dry_run: args.is_dry_run,
    },
  )
  .await?;
  Ok(Some(call))
}
