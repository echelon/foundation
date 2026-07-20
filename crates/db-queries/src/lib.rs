//! The ONLY SQL in the codebase (codebase_layout_backend.md §4). One function per query,
//! grouped by entity. Every function takes an `impl sqlx::PgExecutor` so it composes with
//! either a pool (autocommit) or a `&mut Transaction` (the signup flow, which must be atomic —
//! accounts.md §3).
//!
//! Row structs expose only the columns callers need; `sid` (identity) is never selected.
//! (These are runtime `query_as` calls; wiring `.sqlx/` offline compile-checking is a
//! follow-up — codebase_layout_backend.md §7.)

pub mod accounts;
pub mod invitations;
pub mod members;
pub mod rows;
pub mod sessions;
pub mod sso;
pub mod users;

pub use rows::*;
