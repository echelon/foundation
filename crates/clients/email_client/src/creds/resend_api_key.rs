/// Holds the Resend API key. Sent on every request as `Authorization: Bearer <api_key>`.
#[derive(Clone)]
pub struct ResendApiKey {
  pub(crate) api_key: String,
}

impl ResendApiKey {
  pub fn new(api_key: impl Into<String>) -> Self {
    Self { api_key: api_key.into() }
  }
}

// Debug is redacted so the key can't end up in a log line via accidental `{:?}` on an enclosing
// args struct.
impl std::fmt::Debug for ResendApiKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "ResendApiKey(<redacted>)")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn debug_is_redacted() {
    let key = ResendApiKey::new("super_secret_value_must_not_leak");
    let rendered = format!("{:?}", key);
    assert_eq!(rendered, "ResendApiKey(<redacted>)");
    assert!(!rendered.contains("super_secret_value_must_not_leak"));
  }

  #[test]
  fn accepts_str_and_string() {
    let _from_str = ResendApiKey::new("abc");
    let _from_string = ResendApiKey::new(String::from("abc"));
  }
}
