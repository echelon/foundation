//! Inbound callback ("webhook") parsing.
//!
//! Kling POSTs to the `callback_url` given at create time on every task
//! status change. There are two payload dialects:
//!
//! - **New-style** (model-version-in-path endpoints — all video bindings):
//!   the body IS a full task record → [`parse_task_callback`].
//! - **Legacy-style** (`/v1/*` endpoints — image, audio, element and voice
//!   management): the `task_id`/`task_status` shape →
//!   [`parse_legacy_callback`].
//!
//! Kling callbacks carry **no signature** — treat them strictly as hints:
//! match on `external_id`/`external_task_id`, then confirm the state by
//! polling before any state transition.

pub mod parse_legacy_callback;
pub mod parse_task_callback;

pub use parse_legacy_callback::{parse_legacy_callback, LegacyCallback, LegacyCallbackResult};
pub use parse_task_callback::parse_task_callback;
