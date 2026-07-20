//! Row structs returned by the query functions. Only exposed columns — never `sid`.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
  pub id: String,
  pub email: String,
  pub email_clean: String,
  pub email_original: String,
  pub email_verified_at: Option<DateTime<Utc>>,
  pub password_hash: Option<String>,
  pub session_version: i32,
  pub display_name: Option<String>,
  pub is_staff: bool,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Account {
  pub id: String,
  pub name: Option<String>,
  pub status: String,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AccountMember {
  pub id: String,
  pub account_id: String,
  pub user_id: String,
  pub role: String,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Session {
  pub id: String,
  pub user_id: String,
  pub impersonator_user_id: Option<String>,
  pub created_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
  pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SsoConnection {
  pub id: String,
  pub user_id: String,
  pub provider: String,
  pub provider_user_id: String,
  pub provider_email: String,
  pub created_at: DateTime<Utc>,
  pub last_used_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Invitation {
  pub id: String,
  pub account_id: String,
  pub invited_by_user_id: String,
  pub accepted_by_user_id: Option<String>,
  pub email: String,
  pub role: String,
  pub code: String,
  pub token_hash: String,
  pub created_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
  pub accepted_at: Option<DateTime<Utc>>,
  pub revoked_at: Option<DateTime<Utc>>,
}
