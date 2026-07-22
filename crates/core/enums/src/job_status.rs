use crate::text_enum::text_enum;

text_enum! {
  /// Job lifecycle (jobs.md §2) — external == internal, closed set.
  pub enum JobStatus {
    Queued => "queued",
    Running => "running",
    Succeeded => "succeeded",
    Failed => "failed",
    Canceled => "canceled",
    Expired => "expired",
  }
}

impl JobStatus {
  pub fn is_terminal(&self) -> bool {
    matches!(self, Self::Succeeded | Self::Failed | Self::Canceled | Self::Expired)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["queued", "running", "succeeded", "failed", "canceled", "expired"];
    assert_eq!(JobStatus::ALL.len(), expect.len());
    for (v, s) in JobStatus::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<JobStatus>().unwrap(), *v);
    }
  }
}
