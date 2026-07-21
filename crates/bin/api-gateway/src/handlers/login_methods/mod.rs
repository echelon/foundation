//! Login methods (accounts.md §5/§6): ONE inclusive resource — the password entry (never
//! deletable, so it exposes no id) alongside the SSO connections. Managed from settings.

mod link_sso_handler;
mod list_login_methods_handler;
mod unlink_sso_handler;
pub mod types;

pub use link_sso_handler::*;
pub use list_login_methods_handler::*;
pub use unlink_sso_handler::*;
