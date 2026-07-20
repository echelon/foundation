//! Email client — transactional email via **Resend** (email.md §1). A deliberately thin wrapper
//! over Resend's REST API (`POST https://api.resend.com/emails`), built on the shared
//! [`client_core::http_client`] like every other provider client.
//!
//! This crate only knows how to *send one email* — it takes a fully-rendered
//! [`api::send_email::EmailMessage`] (`from`, `to`, `subject`, `html`) and returns the provider
//! message id. Templating (subject/body/links), the `email_sends` audit row, and idempotency all
//! live in the caller (api-gateway / the future email worker), per email.md §2.
//!
//! Auth is `Authorization: Bearer <key>`; the key lives in [`creds::resend_api_key::ResendApiKey`]
//! whose `Debug` is redacted so it can't leak through an enclosing `{:?}`.

pub mod api;
pub mod creds;
pub mod error;

pub use api::send_email::{send_email, EmailMessage, EmailSendReceipt, SendEmailArgs};
pub use creds::resend_api_key::ResendApiKey;
pub use error::email_error::EmailError;
