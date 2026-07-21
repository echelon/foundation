//! HTTP handlers — ONE handler per file (`{area}/{name}_handler.rs`), each with typed,
//! utoipa-documented request/response structs. Never `Json<Value>`.

pub mod api_types;
pub(crate) mod support;

pub mod account;
pub mod api_keys;
pub mod audit_logs;
pub mod auth;
pub mod avatar;
pub mod catalog;
pub mod invitations;
pub mod login_methods;
pub mod org;
pub mod root;
pub mod sessions;
