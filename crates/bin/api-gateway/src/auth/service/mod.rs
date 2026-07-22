//! Auth-flow logic (accounts.md §3–§5, auth.md §2) — handlers are thin wrappers over
//! these. ONE bounded context per file; shared internals live in `support`.
//! Signup/SSO-signup flows run in ONE transaction (user + account + owner membership +
//! session commit atomically — accounts.md §3).

use db_queries::{Session, User};

mod api_keys;
mod business;
mod email;
mod email_verification;
mod invitations;
mod login;
mod members;
mod password;
mod password_reset;
mod profile;
mod sessions;
mod signup;
mod sso;
mod support;

pub use api_keys::*;
pub use business::*;
pub use email::*;
pub use email_verification::*;
pub use invitations::*;
pub use login::*;
pub use members::*;
pub use password::*;
pub use password_reset::*;
pub use profile::*;
pub use sessions::*;
pub use signup::*;
pub use sso::*;
pub(crate) use support::*;

/// Request metadata captured for the session row / audit.
#[derive(Debug, Clone, Default)]
pub struct ReqMeta {
  pub ip: Option<String>,
  pub user_agent: Option<String>,
}

/// The result of any flow that ends logged-in: the user, their new session, and the JWT to
/// set as a cookie.
pub struct AuthOutcome {
  pub user: User,
  pub session: Session,
  pub token: String,
}

