use crate::text_enum::text_enum;

text_enum! {
  /// Audit events (audit_logs.md §3 — the write matrix is the authority). Additive-forever:
  /// new events append; old rows never reinterpret.
  pub enum AuditEvent {
    PasswordChanged => "password_changed",
    PasswordReset => "password_reset",
    EmailChanged => "email_changed",
    EmailVerified => "email_verified",
    DisplayNameChanged => "display_name_changed",
    SsoConnected => "sso_connected",
    SsoDisconnected => "sso_disconnected",
    SessionRevoked => "session_revoked",
    AccountCreated => "account_created",
    MemberAdded => "member_added",
    MemberRoleChanged => "member_role_changed",
    MemberRemoved => "member_removed",
    OwnershipTransferred => "ownership_transferred",
    InvitationSent => "invitation_sent",
    InvitationRevoked => "invitation_revoked",
    InvitationAccepted => "invitation_accepted",
    ApiKeyCreated => "api_key_created",
    ApiKeyRevoked => "api_key_revoked",
    GatewayProjectKeyAttached => "gateway_project_key_attached",
    BusinessNameChanged => "business_name_changed",
    BillingAddressChanged => "billing_address_changed",
    SpendLimitChanged => "spend_limit_changed",
    AutoRefillChanged => "auto_refill_changed",
    WebhookSecretRotated => "webhook_secret_rotated",
    GatewayProjectCreated => "gateway_project_created",
    GatewayProjectGrantChanged => "gateway_project_grant_changed",
    GatewayWebhookChanged => "gateway_webhook_changed",
    GatewayRoutingPolicyChanged => "gateway_routing_policy_changed",
    GatewayProviderCredentialSaved => "gateway_provider_credential_saved",
    GatewayProviderCredentialRevoked => "gateway_provider_credential_revoked",
    GatewaySupplierSettlementRecorded => "gateway_supplier_settlement_recorded",
    GatewayRequestContentRestoreQueued => "gateway_request_content_restore_queued",
    GatewayRequestContentRemoved => "gateway_request_content_removed",
    GatewayDispatchControlChanged => "gateway_dispatch_control_changed",
    GatewayProviderRecheckScheduled => "gateway_provider_recheck_scheduled",
    GatewayClientBudgetChanged => "gateway_client_budget_changed",
  }
}

impl AuditEvent {
  /// Whether a `user_audit_logs` row for this event is visible to the organizations (accounts)
  /// the user is attached to (audit_logs.md §3 — the Org column; exhaustive on purpose so a
  /// new event forces a decision here).
  pub const fn is_org_visible(&self) -> bool {
    match self {
      Self::PasswordChanged
      | Self::PasswordReset
      | Self::EmailChanged
      | Self::EmailVerified
      | Self::DisplayNameChanged
      | Self::SsoConnected
      | Self::SsoDisconnected => true,

      Self::SessionRevoked
      | Self::AccountCreated
      | Self::MemberAdded
      | Self::MemberRoleChanged
      | Self::MemberRemoved
      | Self::OwnershipTransferred
      | Self::InvitationSent
      | Self::InvitationRevoked
      | Self::InvitationAccepted
      | Self::ApiKeyCreated
      | Self::ApiKeyRevoked
      | Self::GatewayProjectKeyAttached
      | Self::BusinessNameChanged
      | Self::BillingAddressChanged
      | Self::SpendLimitChanged
      | Self::AutoRefillChanged
      | Self::WebhookSecretRotated
      | Self::GatewayProjectCreated
      | Self::GatewayProjectGrantChanged
      | Self::GatewayWebhookChanged
      | Self::GatewayRoutingPolicyChanged
      | Self::GatewayProviderCredentialSaved
      | Self::GatewayProviderCredentialRevoked
      | Self::GatewaySupplierSettlementRecorded
      | Self::GatewayRequestContentRestoreQueued
      | Self::GatewayRequestContentRemoved
      | Self::GatewayDispatchControlChanged
      | Self::GatewayProviderRecheckScheduled
      | Self::GatewayClientBudgetChanged => false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings_roundtrip() {
    for v in AuditEvent::ALL {
      assert_eq!(v.as_str().parse::<AuditEvent>().unwrap(), *v);
      assert_eq!(serde_json::to_string(v).unwrap(), format!("\"{}\"", v.as_str()));
    }
    assert_eq!(AuditEvent::ALL.len(), 36);
  }

  #[test]
  fn org_visibility_matches_the_matrix() {
    assert!(AuditEvent::PasswordChanged.is_org_visible());
    assert!(AuditEvent::EmailChanged.is_org_visible());
    assert!(AuditEvent::DisplayNameChanged.is_org_visible());
    assert!(!AuditEvent::SessionRevoked.is_org_visible()); // user-private
    assert!(!AuditEvent::AccountCreated.is_org_visible()); // reaches the org as an A-row instead
  }
}
