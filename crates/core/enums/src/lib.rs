//! The shared enum registry (enums.md). Every closed string vocabulary that crosses the API or
//! the database is a REAL Rust enum declared here — **never** a stringly-typed `&str`/`String`.
//!
//! Each enum is TEXT-backed: the database stores the wire string (CHECK-constrained in the
//! schema), serde serializes the same string, and sqlx encodes/decodes it directly, so
//! `query_as!` column overrides (`AS "col: enums::AvatarSource"`) and binds just work. Wire
//! strings are explicit per-variant (no rename_all magic) and pinned by golden tests.

mod text_enum;

mod audit_event;
mod avatar_file_source;
mod avatar_source;
mod email_status;
mod session_creation_type;
mod email_template;
mod monogram_gradient;

pub use audit_event::AuditEvent;
pub use avatar_file_source::AvatarFileSource;
pub use avatar_source::AvatarSource;
pub use email_status::EmailStatus;
pub use session_creation_type::SessionCreationType;
pub use email_template::EmailTemplate;
pub use monogram_gradient::MonogramGradient;

/// Error parsing an enum from an unknown wire string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEnumError {
  pub enum_name: &'static str,
  pub got: String,
}

impl std::fmt::Display for ParseEnumError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "unknown {} value: {:?}", self.enum_name, self.got)
  }
}

impl std::error::Error for ParseEnumError {}
