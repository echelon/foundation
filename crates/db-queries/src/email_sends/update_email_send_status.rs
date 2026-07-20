use enums::EmailStatus;
use ids::EmailSendId;
use sqlx::PgExecutor;

/// Advance a send's lifecycle after calling the provider: set `status`, and (optionally) the
/// `provider_message_id` on success or `error` on failure. Returns rows affected.
pub async fn update_email_send_status<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &EmailSendId,
  status: EmailStatus,
  provider_message_id: Option<&str>,
  error: Option<&str>,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE email_sends
       SET status = $2,
           provider_message_id = COALESCE($3, provider_message_id),
           error = $4,
           updated_at = now()
       WHERE id = $1"#,
    id.as_str(),
    status.as_str(),
    provider_message_id,
    error,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
