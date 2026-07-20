//! The ONLY SQL in the codebase (codebase_layout_backend.md §4). Queries are grouped by table,
//! **one query per file**; each table module also declares its row struct.
//!
//! RULES (see `CLAUDE.md`): always use the compile-checked `sqlx::query!` / `query_as!` /
//! `query_scalar!` macros (never runtime string queries), validated against the `.sqlx/` offline
//! cache. Ids are strongly typed (`ids::UserId`, …) — columns decode via `AS "col: Type"`
//! overrides, params bind `.as_str()`. Every fn is executor-generic so it composes with a pool
//! or a `&mut Transaction`.

pub mod accounts;
pub mod invitations;
pub mod members;
pub mod sessions;
pub mod sso;
pub mod users;

// Row structs are re-exported at the crate root for ergonomic imports by consumers.
pub use accounts::Account;
pub use invitations::Invitation;
pub use members::AccountMember;
pub use sessions::Session;
pub use sso::SsoConnection;
pub use users::User;
