use crate::text_enum::text_enum;

text_enum! {
  /// Append-only job trail entries (jobs.md).
  pub enum JobEventType {
    Created => "created",
    Charged => "charged",
    DispatchAttempted => "dispatch_attempted",
    ProviderAccepted => "provider_accepted",
    PollTick => "poll_tick",
    ProviderCompleted => "provider_completed",
    OutputIngested => "output_ingested",
    WebhookSent => "webhook_sent",
    Refunded => "refunded",
    Failed => "failed",
    Canceled => "canceled",
    Expired => "expired",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["created", "charged", "dispatch_attempted", "provider_accepted", "poll_tick", "provider_completed", "output_ingested", "webhook_sent", "refunded", "failed", "canceled", "expired"];
    assert_eq!(JobEventType::ALL.len(), expect.len());
    for (v, s) in JobEventType::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<JobEventType>().unwrap(), *v);
    }
  }
}
