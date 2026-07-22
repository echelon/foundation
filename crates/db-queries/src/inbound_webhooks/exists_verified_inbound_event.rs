use sqlx::PgExecutor;

/// Was a signature-valid delivery of this provider event already recorded with a
/// non-duplicate outcome? (Provider webhooks are at-least-once — redeliveries are normal.)
pub async fn exists_verified_inbound_event<'e, E: PgExecutor<'e>>(
  exec: E,
  provider: &str,
  event_id: &str,
) -> sqlx::Result<bool> {
  let row = sqlx::query_scalar!(
    r#"SELECT EXISTS (
         SELECT 1 FROM inbound_webhooks
         WHERE provider = $1 AND event_id = $2 AND is_signature_valid AND outcome <> 'duplicate'
       ) AS "exists!""#,
    provider,
    event_id,
  )
  .fetch_one(exec)
  .await?;
  Ok(row)
}
