use crate::text_enum::text_enum;

text_enum! {
  /// Where a queued job is waiting (jobs.md two-stage queue).
  pub enum QueueStage {
    AtOurGateway => "at_our_gateway",
    AtProvider => "at_provider",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["at_our_gateway", "at_provider"];
    assert_eq!(QueueStage::ALL.len(), expect.len());
    for (v, s) in QueueStage::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<QueueStage>().unwrap(), *v);
    }
  }
}
