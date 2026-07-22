use crate::text_enum::text_enum;

text_enum! {
  /// What the gateway did with an inbound provider webhook (`inbound_webhooks.outcome`,
  /// logging.md addendum 2026-07-22). Variants: `processed` — verified, correlated, and the
  /// job/slice state advanced; `unmatched` — verified but no job/slice claims the subject
  /// id; `duplicate` — a verified delivery of an event id we already handled (at-least-once
  /// redelivery); `ignored` — verified but an event family we don't act on;
  /// `signature_failed` — rejected at the door; `not_configured` — arrived with no webhook
  /// secret configured, could not be verified; `error` — verified and matched but
  /// processing failed (the poll loop remains the safety net).
  pub enum InboundWebhookOutcome {
    Processed => "processed",
    Unmatched => "unmatched",
    Duplicate => "duplicate",
    Ignored => "ignored",
    SignatureFailed => "signature_failed",
    NotConfigured => "not_configured",
    Error => "error",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["processed", "unmatched", "duplicate", "ignored", "signature_failed", "not_configured", "error"];
    assert_eq!(InboundWebhookOutcome::ALL.len(), expect.len());
    for (v, s) in InboundWebhookOutcome::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<InboundWebhookOutcome>().unwrap(), *v);
    }
  }
}
