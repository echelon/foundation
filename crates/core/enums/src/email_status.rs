use crate::text_enum::text_enum;

text_enum! {
  /// Lifecycle of an `email_sends` row (email.md §4).
  pub enum EmailStatus {
    Queued => "queued",
    Sent => "sent",
    Delivered => "delivered",
    Bounced => "bounced",
    Complained => "complained",
    Failed => "failed",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["queued", "sent", "delivered", "bounced", "complained", "failed"];
    for (v, s) in EmailStatus::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<EmailStatus>().unwrap(), *v);
    }
  }
}
