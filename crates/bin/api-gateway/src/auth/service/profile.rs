//! Display-name / profile updates.


use db_queries::{
  user_avatars, users, User,
};
use enums::AuditEvent;

use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// Change the user's display name (nullable — empty clears it). Atomically: update `users`,
/// refresh the avatar's person-letter monogram, and write the ORG-VISIBLE
/// `display_name_changed` audit event (audit_logs.md §3 — accounts the user belongs to can see
/// it). Returns the new name.
pub async fn change_display_name(
  pool: &db::PgPool,
  user: &User,
  new_name: Option<&str>,
  ip: Option<&str>,
) -> ApiResult<Option<String>> {
  let new_name = new_name.map(str::trim).filter(|s| !s.is_empty());
  if let Some(name) = new_name {
    if name.len() > 100 {
      return Err(ApiError::BadRequest("name must be 100 characters or fewer".into()));
    }
  }
  if new_name == user.display_name.as_deref() {
    return Ok(user.display_name.clone()); // no-op; no audit noise
  }

  let (first, last) = person_letters(new_name);
  let mut tx = pool.begin().await?;
  users::update_display_name(&mut *tx, &user.id, new_name).await?;
  user_avatars::set_person_letters(&mut *tx, &user.id, first.as_deref(), last.as_deref()).await?;
  let data = serde_json::json!({ "from": user.display_name, "to": new_name });
  write_user_event(&mut *tx, &user.id, AuditEvent::DisplayNameChanged, Some(data), ip).await?;
  tx.commit().await?;
  Ok(new_name.map(str::to_string))
}
