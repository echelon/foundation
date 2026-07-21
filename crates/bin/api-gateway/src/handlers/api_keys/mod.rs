//! API-key management (auth.md §4, revised 2026-07-21: ANY account member may create and
//! delete keys). The plaintext secret is returned exactly once, at creation.

mod create_api_key_handler;
mod delete_api_key_handler;
mod list_api_keys_handler;
pub mod types;

pub use create_api_key_handler::*;
pub use delete_api_key_handler::*;
pub use list_api_keys_handler::*;
