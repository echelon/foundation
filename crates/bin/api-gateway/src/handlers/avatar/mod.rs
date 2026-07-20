//! Avatar handlers (accounts.md §2): upload/replace with an optional crop, remove the custom
//! avatar, choose what displays, and dismiss/refresh the gravatar.
//!
//! The gateway is NOT a CDN — no handler here serves image bytes. Every response carries the
//! avatar's [`crate::avatar_links::AvatarInfo`] (display source + CDN links derived from stored
//! bucket-path pieces), and `/v1/auth/me` reports the same shape, so clients always render from
//! the CDN.

mod refresh_my_gravatar_handler;
mod remove_my_avatar_handler;
mod remove_my_gravatar_handler;
mod set_my_avatar_preference_handler;
pub mod support;
mod upload_my_avatar_handler;

pub use refresh_my_gravatar_handler::*;
pub use remove_my_avatar_handler::*;
pub use remove_my_gravatar_handler::*;
pub use set_my_avatar_preference_handler::*;
pub use upload_my_avatar_handler::*;
