use crate::text_enum::text_enum;

text_enum! {
  /// Which door created the job (jobs.md).
  pub enum JobSource {
    ApiKey => "api_key",
    Dashboard => "dashboard",
    Mcp => "mcp",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["api_key", "dashboard", "mcp"];
    assert_eq!(JobSource::ALL.len(), expect.len());
    for (v, s) in JobSource::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<JobSource>().unwrap(), *v);
    }
  }
}
