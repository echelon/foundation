//! The TYPED union of every audit-log `data` payload this codebase writes
//! (request/response structs never carry bare `serde_json::Value`). All fields
//! optional; each event sets only what's relevant. Adding a new payload key
//! means adding a field HERE — never an ad-hoc `json!` at a write site.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct AuditData {
  /// SSO provider (link/unlink events).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub provider: Option<String>,
  /// Membership/invitation role (member + invitation events).
  #[serde(skip_serializing_if = "Option::is_none")]
  #[schema(value_type = Option<String>, example = "member")]
  pub role: Option<enums::MemberRole>,
  /// API key title (key mint events).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  /// API key display prefix (key mint events).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub key_prefix: Option<String>,
  /// Subject email (invitation events).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub email: Option<String>,
  /// A prior invitation was superseded by this one.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub superseded: Option<bool>,
  /// Previous value (business-name changes; null = placeholder was shown).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub from: Option<String>,
  /// New value (business-name changes).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub to: Option<String>,
  /// Related account (cross-account membership events).
  #[serde(skip_serializing_if = "Option::is_none")]
  #[schema(value_type = Option<String>, example = "acct_01k0…")]
  pub account_id: Option<ids::AccountId>,
}

impl AuditData {
  /// Serialize for the jsonb storage boundary (a plain optional-field struct —
  /// serialization cannot fail).
  pub fn value(&self) -> serde_json::Value {
    serde_json::to_value(self).expect("AuditData is always serializable")
  }

  /// Parse a stored payload; rows this codebase wrote always parse.
  pub fn parse(value: serde_json::Value) -> Option<AuditData> {
    serde_json::from_value(value).ok()
  }
}
