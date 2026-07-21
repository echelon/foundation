//! Authentication: password hashing, email canonicalization, the session-token (JWT),
//! cookie helpers, and the flow logic (`service`). Handlers stay thin over this.

pub mod cookie;
pub mod email;
pub mod github;
pub mod google;
pub mod password;
pub mod service;
pub mod sso_verify;
pub mod token;
