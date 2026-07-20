/// Holds the OpenAI API key. Sent on every request as `Authorization: Bearer <api_key>`.
///
/// Optionally scopes to an organization / project via the `OpenAI-Organization` /
/// `OpenAI-Project` headers.
#[derive(Clone)]
pub struct OpenAiApiKey {
  pub(crate) api_key: String,
  pub(crate) organization: Option<String>,
  pub(crate) project: Option<String>,
}

impl OpenAiApiKey {
  pub fn new(api_key: impl Into<String>) -> Self {
    Self { api_key: api_key.into(), organization: None, project: None }
  }

  pub fn with_organization(mut self, org: impl Into<String>) -> Self {
    self.organization = Some(org.into());
    self
  }

  pub fn with_project(mut self, project: impl Into<String>) -> Self {
    self.project = Some(project.into());
    self
  }
}

// Debug is redacted so the key can't leak via a `{:?}` on an enclosing struct.
impl std::fmt::Debug for OpenAiApiKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("OpenAiApiKey")
      .field("api_key", &"<redacted>")
      .field("organization", &self.organization)
      .field("project", &self.project)
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn debug_redacts_the_key() {
    let key = OpenAiApiKey::new("sk-secret-value").with_organization("org_123");
    let dbg = format!("{key:?}");
    assert!(!dbg.contains("sk-secret-value"), "debug leaked the key: {dbg}");
    assert!(dbg.contains("<redacted>"));
    assert!(dbg.contains("org_123"));
  }
}
