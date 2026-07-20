//! Authentication: password hashing, email canonicalization, the session-token (JWT),
//! cookie helpers, and the flow logic (`service`). Handlers stay thin over this.

pub mod cookie;
pub mod email;
pub mod password;
pub mod service;
pub mod token;
