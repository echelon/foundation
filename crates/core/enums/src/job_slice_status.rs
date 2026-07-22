use crate::text_enum::text_enum;

text_enum! {
  /// Fan-out slice lifecycle (jobs.md addendum 2026-07-22).
  pub enum JobSliceStatus {
    Queued => "queued",
    Running => "running",
    Succeeded => "succeeded",
    Failed => "failed",
    Canceled => "canceled",
  }
}

impl JobSliceStatus {
  pub fn is_terminal(&self) -> bool {
    matches!(self, Self::Succeeded | Self::Failed | Self::Canceled)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["queued", "running", "succeeded", "failed", "canceled"];
    assert_eq!(JobSliceStatus::ALL.len(), expect.len());
    for (v, s) in JobSliceStatus::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<JobSliceStatus>().unwrap(), *v);
    }
  }
}
