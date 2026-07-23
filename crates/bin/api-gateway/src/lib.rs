//! api-gateway — all web ingress (codebase_layout_backend.md §2). The library holds the
//! router, handlers, middleware, and auth logic; `main.rs` is a thin binary over [`run`].
//!
//! Module map:
//!   - `state`      — shared `AppState` (pool, jwt keys, cookie config)
//!   - `error`      — `ApiError` → JSON responses
//!   - `auth`       — password, email canonicalization, session JWT, cookies, flow `service`
//!   - `authz`      — account-role checks (auth.md §4)
//!   - `middleware` — the `AuthContext` session extractor (online-checked)
//!   - `handlers`   — thin HTTP handlers, one per file, grouped by area
//!   - `openapi`    — the OpenAPI document (utoipa); exported by the `export-openapi` bin
//!   - `app`        — router assembly + server bootstrap

pub mod app;
pub mod auth;
pub mod audit_data;
pub mod authz;
pub mod avatar_links;
pub mod business_names;
pub mod error;
pub mod generation;
pub mod handlers;
pub mod mailer;
pub mod middleware;
pub mod openapi;
pub mod state;

pub use app::{build_router, run};
pub use state::AppState;
