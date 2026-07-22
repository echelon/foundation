use crate::text_enum::text_enum;

text_enum! {
  /// Which API plane created the job (api_any.md — the four planes).
  pub enum ApiShape {
    Granular => "granular",
    Unified => "unified",
    Any => "any",
    Chat => "chat",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn golden_wire_strings() {
    let expect = ["granular", "unified", "any", "chat"];
    assert_eq!(ApiShape::ALL.len(), expect.len());
    for (v, s) in ApiShape::ALL.iter().zip(expect) {
      assert_eq!(v.as_str(), s);
      assert_eq!(s.parse::<ApiShape>().unwrap(), *v);
    }
  }
}
