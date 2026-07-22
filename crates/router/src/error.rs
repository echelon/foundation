use std::fmt::{Display, Formatter};

/// Router-layer failures: capability mismatches and plan validation, plus wrapped provider
/// client errors (kept as strings at this layer — the provider crates own the rich types).
#[derive(Debug)]
pub enum RouterError {
  /// The catalog slug doesn't route to any implementation.
  UnknownModel(String),
  /// The plan asked for something this model cannot do.
  Invalid(String),
  /// The provider client failed (stringified `OpenAiError` etc. — logged upstream).
  Provider(String),
}

impl std::error::Error for RouterError {}

impl Display for RouterError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::UnknownModel(slug) => write!(f, "no route for model: {slug}"),
      Self::Invalid(msg) => write!(f, "invalid generation plan: {msg}"),
      Self::Provider(msg) => write!(f, "provider error: {msg}"),
    }
  }
}
