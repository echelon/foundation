//! Outbound email (email.md). Renders a template, records an `email_sends` row, and sends via the
//! Resend [`email_client`]. Best-effort: a missing key or a provider error is logged and the send
//! row marked `failed` — it never fails the calling request (important for the reset flow, which
//! must not reveal whether an address exists).
//!
//! NOTE: the design calls for a `send_email` queue task consumed by the worker (email.md §2). Until
//! that worker exists we send inline here so the flows are testable end-to-end; the seam (this
//! module) is the natural place to swap in an enqueue later.

use db_queries::email_sends;
use enums::{EmailStatus, EmailTemplate};
use email_client::{send_email, EmailMessage, SendEmailArgs};
use ids::UserId;

use crate::state::AppState;

/// Send the password-reset email. `token` is the plaintext (only its hash is stored); the link
/// lands on the frontend's reset page.
pub async fn send_password_reset_email(state: &AppState, user_id: &UserId, to_email: &str, token: &str) {
  let link = format!("{}/reset-password?token={}", state.app_base_url.trim_end_matches('/'), token);
  let subject = "Reset your Foundation password";
  let html = format!(
    r#"<p>We received a request to reset your Foundation password.</p>
       <p><a href="{link}">Click here to choose a new password</a>. This link expires in about an hour and can be used once.</p>
       <p>If the link doesn't work, paste this into your browser:<br>{link}</p>
       <p>If you didn't request this, you can safely ignore this email — your password won't change.</p>
       <p>— The Foundation team</p>"#
  );
  deliver(state, Some(user_id), to_email, EmailTemplate::PasswordReset, subject, &html).await;
}

/// Send the email-verification email. `token` is the plaintext (only its hash is stored); the
/// link lands on the frontend's verify page.
pub async fn send_verification_email(state: &AppState, user_id: &UserId, to_email: &str, token: &str) {
  let link = format!("{}/verify-email?token={}", state.app_base_url.trim_end_matches('/'), token);
  let subject = "Verify your Foundation email";
  let html = format!(
    r#"<p>Confirm this is you: verify your email address for Foundation.</p>
       <p><a href="{link}">Click here to verify your email</a>. This link expires in 24 hours and can be used once.</p>
       <p>If the link doesn't work, paste this into your browser:<br>{link}</p>
       <p>If you didn't request this, you can safely ignore this email.</p>
       <p>— The Foundation team</p>"#
  );
  deliver(state, Some(user_id), to_email, EmailTemplate::EmailVerify, subject, &html).await;
}

/// Everything the invitation email needs (named fields — four `&str`s in a row is exactly the
/// transposition hazard the args-struct rule exists for).
pub struct InvitationEmailArgs<'a> {
  pub to_email: &'a str,
  /// Who invited them (display name, falling back to their email).
  pub inviter: &'a str,
  /// The inviting organization's name (unnamed accounts → generic copy).
  pub account_name: Option<&'a str>,
  pub role: &'a str,
  /// The plaintext magic-link token (only its hash is stored).
  pub token: &'a str,
  /// The short human code (shown for the "join with code" path).
  pub code: &'a str,
}

/// Send the invitation email (email.md §3: template `invite` — carries the magic link AND the
/// short code). `user_id` is None: invites are bound to an email, which may predate any user.
pub async fn send_invitation_email(state: &AppState, args: InvitationEmailArgs<'_>) {
  let link = format!("{}/join?token={}", state.app_base_url.trim_end_matches('/'), args.token);
  let org = args.account_name.map(|n| format!("the {n} organization")).unwrap_or_else(|| "an organization".into());
  let subject = format!("{} invited you to join {org} on Foundation", args.inviter);
  // Inviter and org names are user-controlled — escape them (tokens/codes are server-minted).
  let html = format!(
    r#"<p>{inviter} invited you to join {org} on Foundation as {role}.</p>
       <p><a href="{link}">Click here to accept the invitation</a>. It expires in 7 days.</p>
       <p>If the link doesn't work, paste this into your browser:<br>{link}</p>
       <p>Or go to the join page and enter this code: <b>{code}</b></p>
       <p>If you weren't expecting this, you can safely ignore this email.</p>
       <p>— The Foundation team</p>"#,
    inviter = escape_html(args.inviter),
    org = escape_html(&org),
    role = escape_html(args.role),
    code = args.code,
  );
  deliver(state, None, args.to_email, EmailTemplate::Invite, &subject, &html).await;
}

/// Minimal HTML escape for user-controlled text interpolated into email bodies.
fn escape_html(s: &str) -> String {
  s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Record + attempt one send. Swallows all errors (logs them); the `email_sends` row carries the
/// outcome for later inspection.
async fn deliver(state: &AppState, user_id: Option<&UserId>, to_email: &str, template: EmailTemplate, subject: &str, html: &str) {
  // Log the intent first so we have a durable record even if the process dies mid-send.
  let send_row = email_sends::create_email_send(
    &state.pool,
    email_sends::NewEmailSend { user_id, to_email, template, status: EmailStatus::Queued },
  )
  .await;
  let send_row = match send_row {
    Ok(row) => row,
    Err(e) => {
      tracing::error!(error = %e, "failed to record email_sends row");
      return;
    }
  };

  let Some(api_key) = state.resend_api_key.as_ref() else {
    tracing::warn!(to = %to_email, template = %template, "email not configured (no RESEND_API_KEY) — skipping send");
    return; // row stays `queued`
  };

  let result = send_email(SendEmailArgs {
    api_key,
    message: EmailMessage {
      from: state.email_from.clone(),
      to: to_email.to_string(),
      subject: subject.to_string(),
      html: html.to_string(),
    },
  })
  .await;

  let _ = match result {
    Ok(receipt) => {
      email_sends::update_email_send_status(&state.pool, &send_row.id, EmailStatus::Sent, Some(&receipt.provider_message_id), None)
        .await
    }
    Err(e) => {
      tracing::error!(error = %e, to = %to_email, template = %template, "email send failed");
      email_sends::update_email_send_status(&state.pool, &send_row.id, EmailStatus::Failed, None, Some(&e.to_string())).await
    }
  };
}
