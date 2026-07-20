/// Holds the API key for authenticating with the Kinovi API.
///
/// Sent on every request as `Authorization: Bearer <api_key>`.
#[derive(Clone)]
pub struct KinoviApiKey {
  pub(crate) api_key: String,
}

impl KinoviApiKey {
  pub fn new(api_key: impl Into<String>) -> Self {
    Self { api_key: api_key.into() }
  }
}

// Debug is redacted so the key can't end up in a log line via accidental
// `{:?}` on an enclosing args struct.
impl std::fmt::Debug for KinoviApiKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "KinoviApiKey(<redacted>)")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn debug_is_redacted() {
    let key = KinoviApiKey::new("super_secret_value_must_not_leak");
    let rendered = format!("{:?}", key);
    assert_eq!(rendered, "KinoviApiKey(<redacted>)");
    assert!(!rendered.contains("super_secret_value_must_not_leak"));
  }

  #[test]
  fn accepts_str_and_string() {
    let _from_str = KinoviApiKey::new("abc");
    let _from_string = KinoviApiKey::new(String::from("abc"));
  }
}
