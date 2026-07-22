use crate::text_enum::text_enum;

text_enum! {
  /// Who canceled a job (jobs.md).
  pub enum CanceledBy {
    User => "user",
    System => "system",
    Provider => "provider",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["user", "system", "provider"];
    assert_eq!(CanceledBy::ALL.len(), expect.len());
    for (v, s) in CanceledBy::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<CanceledBy>().unwrap(), *v);
    }
  }
}
