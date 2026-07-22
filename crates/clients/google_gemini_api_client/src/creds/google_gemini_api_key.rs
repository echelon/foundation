/// Holds the API key for authenticating with the Google Gemini API.
///
/// Sent on every request as the `x-goog-api-key: <api_key>` header.
#[derive(Clone)]
pub struct GoogleGeminiApiKey {
  pub(crate) api_key: String,
}

impl GoogleGeminiApiKey {
  pub fn new(api_key: String) -> Self {
    Self { api_key }
  }
}

// Debug is redacted so the key can't end up in a log line via accidental
// `{:?}` on an enclosing args struct.
impl std::fmt::Debug for GoogleGeminiApiKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "GoogleGeminiApiKey(<redacted>)")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn debug_is_redacted() {
    let key = GoogleGeminiApiKey::new("super_secret_value".to_string());
    let rendered = format!("{:?}", key);
    assert_eq!(rendered, "GoogleGeminiApiKey(<redacted>)");
    assert!(!rendered.contains("super_secret_value"));
  }
}
