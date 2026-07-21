use enums::MonogramGradient;
use ids::UserId;
use sqlx::PgExecutor;

/// Pick the initials-monogram gradient. Returns whether the user's avatar row existed.
pub async fn set_monogram_gradient<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  gradient: MonogramGradient,
) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars SET monogram_gradient = $2, updated_at = now() WHERE user_id = $1"#,
    user_id.as_str(),
    gradient.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
