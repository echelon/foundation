use ids::ProviderCallId;
use sqlx::PgExecutor;

#[derive(Debug)]
pub struct ProviderCallResult<'a> {
  pub id: &'a ProviderCallId,
  /// The provider's handle from the response (e.g. OpenAI `resp_…`) — the correlation key
  /// inbound webhooks look up.
  pub provider_job_id: Option<&'a str>,
  pub response_body: Option<&'a serde_json::Value>,
  pub latency_ms: Option<i32>,
}

/// Record what came back (or the error) on the pre-send row.
pub async fn set_provider_call_result<'e, E: PgExecutor<'e>>(
  exec: E,
  result: ProviderCallResult<'_>,
) -> sqlx::Result<()> {
  sqlx::query!(
    r#"UPDATE provider_calls
       SET provider_job_id = $2, response_body = $3, latency_ms = $4
       WHERE id = $1"#,
    result.id.as_str(),
    result.provider_job_id,
    result.response_body,
    result.latency_ms,
  )
  .execute(exec)
  .await?;
  Ok(())
}
