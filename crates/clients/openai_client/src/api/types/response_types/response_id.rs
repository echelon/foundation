/// A background response id (`resp_…`) — the handle every start binding returns and every
/// poll takes. Newtyped so it can't be transposed with other id-shaped strings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResponseId(pub String);

impl ResponseId {
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl std::fmt::Display for ResponseId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn displays_the_raw_id() {
    assert_eq!(ResponseId("resp_123".into()).to_string(), "resp_123");
  }
}
