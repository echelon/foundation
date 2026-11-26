//! Async access to the TokenCraft native gateway from a trusted Rust service.

mod client;
mod content;
mod error;
mod models;
mod trace;
mod webhooks;

pub use client::TokenCraftClient;
pub use content::*;
pub use error::Error;
pub use models::*;
pub use trace::*;
pub use webhooks::{
    verify_webhook, VerificationError, VerifiedWebhook, WebhookData, WebhookKind, WebhookOptions,
};

fn valid_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        const DIGITS: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";
        suffix.len() == 26 && suffix.bytes().all(|byte| DIGITS.contains(&byte))
    })
}
