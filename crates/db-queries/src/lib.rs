//! The ONLY SQL in the codebase (codebase_layout_backend.md §4). Queries are grouped by table,
//! **one query per file**; each table module also declares its row struct.
//!
//! RULES (see `CLAUDE.md`): always use the compile-checked `sqlx::query!` / `query_as!` /
//! `query_scalar!` macros (never runtime string queries), validated against the `.sqlx/` offline
//! cache. Ids are strongly typed (`ids::UserId`, …) — columns decode via `AS "col: Type"`
//! overrides, params bind `.as_str()`. Every fn is executor-generic so it composes with a pool
//! or a `&mut Transaction`.

pub mod account_audit_logs;
pub mod accounts;
pub mod email_changes;
pub mod email_sends;
pub mod email_verifications;
pub mod invitations;
pub mod members;
pub mod password_resets;
pub mod sessions;
pub mod sso;
pub mod user_audit_logs;
pub mod user_avatar_files;
pub mod user_avatars;
pub mod users;

// Row structs are re-exported at the crate root for ergonomic imports by consumers.
pub use account_audit_logs::AccountAuditLog;
pub use accounts::Account;
pub use email_changes::EmailChange;
pub use email_sends::EmailSend;
pub use email_verifications::EmailVerification;
pub use invitations::Invitation;
pub use members::AccountMember;
pub use password_resets::PasswordReset;
pub use sessions::Session;
pub use sso::SsoConnection;
pub use user_audit_logs::UserAuditLog;
pub use user_avatar_files::UserAvatarFile;
pub use user_avatars::UserAvatar;
pub use users::User;
