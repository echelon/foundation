//! HTTP handlers — thin wrappers over `auth::service` / `db_queries`, grouped by area.

pub mod account_handlers;
pub mod auth_handlers;
pub mod catalog_handlers;
pub mod invitation_handlers;
pub mod root_handlers;
