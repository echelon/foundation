use base64::Engine;

/// The signing secret for a project webhook endpoint (`whsec_…`, from the dashboard's
/// webhook settings). Used to verify inbound webhook signatures — never sent anywhere.
#[derive(Clone)]
pub struct OpenAiWebhookSecret {
  /// The decoded HMAC key (the base64 payload after the `whsec_` prefix).
  pub(crate) key: Vec<u8>,
}

impl OpenAiWebhookSecret {
  /// Accepts the dashboard form (`whsec_<base64>`) or a bare base64 secret.
  pub fn new(secret: impl AsRef<str>) -> Result<Self, String> {
    let raw = secret.as_ref().trim();
    let b64 = raw.strip_prefix("whsec_").unwrap_or(raw);
    let key = base64::engine::general_purpose::STANDARD
      .decode(b64)
      .map_err(|e| format!("webhook secret is not valid base64: {e}"))?;
    if key.is_empty() {
      return Err("webhook secret is empty".into());
    }
    Ok(Self { key })
  }
}

// Debug is redacted so the secret can't leak via `{:?}`.
impl std::fmt::Debug for OpenAiWebhookSecret {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("OpenAiWebhookSecret").field("key", &"<redacted>").finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_prefixed_and_bare_forms() {
    let a = OpenAiWebhookSecret::new("whsec_c2VjcmV0LWtleQ==").unwrap();
    let b = OpenAiWebhookSecret::new("c2VjcmV0LWtleQ==").unwrap();
    assert_eq!(a.key, b"secret-key");
    assert_eq!(a.key, b.key);
  }

  #[test]
  fn rejects_garbage() {
    assert!(OpenAiWebhookSecret::new("whsec_!!!not-base64!!!").is_err());
    assert!(OpenAiWebhookSecret::new("whsec_").is_err());
  }

  #[test]
  fn debug_redacts() {
    let dbg = format!("{:?}", OpenAiWebhookSecret::new("whsec_c2VjcmV0LWtleQ==").unwrap());
    assert!(!dbg.contains("secret"), "{dbg}");
    assert!(dbg.contains("<redacted>"));
  }
}
