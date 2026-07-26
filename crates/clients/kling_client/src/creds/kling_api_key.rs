/// A Kling API key, sent as `Authorization: Bearer <api-key>` on every request.
///
/// This is the current-standard auth for all model-version-in-path endpoints
/// (and the `/v1/general/*` element/voice management endpoints). The legacy
/// AccessKey/SecretKey JWT mint only applies to legacy `model_name`-in-body
/// endpoints, which this crate does not bind.
#[derive(Clone)]
pub struct KlingApiKey {
  pub(crate) api_key: String,
}

impl KlingApiKey {
  pub fn new(api_key: impl Into<String>) -> Self {
    Self { api_key: api_key.into() }
  }
}

// Never print the key — redact in Debug so keys can't leak via logs.
impl std::fmt::Debug for KlingApiKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "KlingApiKey(<redacted>)")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn debug_output_redacts_the_key() {
    let key = KlingApiKey::new("super_secret_value");
    let debug = format!("{key:?}");
    assert!(!debug.contains("super_secret_value"), "got: {debug}");
    assert!(debug.contains("<redacted>"), "got: {debug}");
  }
}
