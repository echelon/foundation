use client_core::http_client;
use serde::Serialize;
use tracing::info;

use crate::creds::resend_api_key::ResendApiKey;
use crate::error::email_error::EmailError;

/// Base URL for the Resend email API.
const RESEND_API_BASE_URL: &str = "https://api.resend.com";
/// User-Agent sent on every Resend request.
const EMAIL_USER_AGENT: &str = "foundation/email-client";

/// A fully-rendered email ready to hand to the provider. Templating happens in the caller — this
/// crate only transports what it's given.
#[derive(Clone, Debug)]
pub struct EmailMessage {
  /// Sender in display form, e.g. `"Foundation <noreply@example.com>"`. Must be a Resend-verified
  /// sender domain.
  pub from: String,
  /// Recipient address (canonicalized by the caller).
  pub to: String,
  pub subject: String,
  /// HTML body.
  pub html: String,
}

/// Top-level argument to [`send_email`]. Borrows the API key separately from the message so callers
/// can log/persist the [`EmailMessage`] without leaking the credential.
#[derive(Clone, Debug)]
pub struct SendEmailArgs<'a> {
  pub api_key: &'a ResendApiKey,
  pub message: EmailMessage,
}

/// What Resend gives back on success: the provider message id, stored on the `email_sends` row as
/// `provider_message_id` (email.md §3).
#[derive(Clone, Debug)]
pub struct EmailSendReceipt {
  pub provider_message_id: String,
}

/// The wire body for `POST /emails`. `to` is an array even for a single recipient.
#[derive(Serialize)]
struct ResendSendBody<'a> {
  from: &'a str,
  to: [&'a str; 1],
  subject: &'a str,
  html: &'a str,
}

#[derive(serde::Deserialize)]
struct ResendSendResponse {
  id: String,
}

/// `POST https://api.resend.com/emails` — send one transactional email. Returns the provider
/// message id. Errors are classified into [`EmailError`]; the caller decides retry/suppression
/// (email.md §2).
pub async fn send_email(args: SendEmailArgs<'_>) -> Result<EmailSendReceipt, EmailError> {
  let SendEmailArgs { api_key, message } = args;

  let body = ResendSendBody {
    from: &message.from,
    to: [&message.to],
    subject: &message.subject,
    html: &message.html,
  };
  // Pre-serialize so a serialization failure surfaces as our error, not swallowed in reqwest.
  let body_json = serde_json::to_string(&body).map_err(EmailError::RequestSerialization)?;

  let url = format!("{RESEND_API_BASE_URL}/emails");
  let client = http_client(EMAIL_USER_AGENT);

  let response = client
    .post(&url)
    .bearer_auth(&api_key.api_key)
    .header("Content-Type", "application/json")
    .body(body_json)
    .send()
    .await
    .map_err(EmailError::Transport)?;

  let status = response.status();
  let response_body = response.text().await.map_err(EmailError::Transport)?;

  info!(status = %status, to = %message.to, subject = %message.subject, "Resend send_email response");

  if !status.is_success() {
    return Err(EmailError::Api { status: status.as_u16(), body: response_body });
  }

  let parsed: ResendSendResponse = serde_json::from_str(&response_body)
    .map_err(|_| EmailError::Api { status: status.as_u16(), body: response_body.clone() })?;

  Ok(EmailSendReceipt { provider_message_id: parsed.id })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample_message() -> EmailMessage {
    EmailMessage {
      from: "Foundation <noreply@example.com>".to_string(),
      to: "user@example.com".to_string(),
      subject: "Reset your password".to_string(),
      html: "<a href=\"https://app/reset?token=abc\">Reset</a>".to_string(),
    }
  }

  #[test]
  fn body_serializes_to_resend_wire_shape() {
    let msg = sample_message();
    let body = ResendSendBody {
      from: &msg.from,
      to: [&msg.to],
      subject: &msg.subject,
      html: &msg.html,
    };
    let json = serde_json::to_string(&body).unwrap();
    assert!(json.contains("\"from\":\"Foundation <noreply@example.com>\""), "got: {json}");
    // `to` is a single-element array.
    assert!(json.contains("\"to\":[\"user@example.com\"]"), "got: {json}");
    assert!(json.contains("\"subject\":\"Reset your password\""), "got: {json}");
  }

  #[test]
  fn body_does_not_serialize_api_key() {
    let key = ResendApiKey::new("secret_value_must_not_leak");
    let args = SendEmailArgs { api_key: &key, message: sample_message() };
    // Only the message is serialized to the wire — never the key.
    let body = ResendSendBody {
      from: &args.message.from,
      to: [&args.message.to],
      subject: &args.message.subject,
      html: &args.message.html,
    };
    let json = serde_json::to_string(&body).unwrap();
    assert!(!json.contains("secret_value_must_not_leak"), "got: {json}");
  }

  #[test]
  fn success_response_parses_message_id() {
    let json = r#"{ "id": "49a3999c-0ce1-4ea6-ab68-afcd6dc2e794" }"#;
    let parsed: ResendSendResponse = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.id, "49a3999c-0ce1-4ea6-ab68-afcd6dc2e794");
  }
}
