//! Standard-Webhooks signature verification (what OpenAI uses): the `webhook-signature`
//! header carries `v1,<base64(HMAC-SHA256(secret, "{webhook-id}.{webhook-timestamp}.{body}"))>`
//! (possibly several space-separated candidates during secret rotation). Verify BEFORE
//! trusting any event — an unauthenticated webhook endpoint is an open command channel.

use base64::Engine;
use sha2::{Digest, Sha256};

use crate::api::webhooks::webhook_event::{parse_webhook_event, WebhookEvent};
use crate::creds::openai_webhook_secret::OpenAiWebhookSecret;
use crate::error::openai_client_error::OpenAiClientError;
use crate::error::openai_error::OpenAiError;

/// Everything from the inbound request (named fields — three header strings in a row is
/// exactly the transposition hazard the args-struct rule exists for).
#[derive(Debug)]
pub struct VerifyWebhookArgs<'a> {
  pub secret: &'a OpenAiWebhookSecret,
  /// The `webhook-id` header.
  pub webhook_id: &'a str,
  /// The `webhook-timestamp` header (unix seconds).
  pub webhook_timestamp: &'a str,
  /// The `webhook-signature` header.
  pub webhook_signature: &'a str,
  /// The RAW request body, exactly as received.
  pub body: &'a str,
}

/// Verify the signature. `Ok(())` = authentic.
pub fn verify_webhook_signature(args: &VerifyWebhookArgs<'_>) -> Result<(), OpenAiClientError> {
  let signed_content = format!("{}.{}.{}", args.webhook_id, args.webhook_timestamp, args.body);
  let expected = hmac_sha256(&args.secret.key, signed_content.as_bytes());

  // The header may carry multiple space-separated signatures (secret rotation).
  for candidate in args.webhook_signature.split_whitespace() {
    let b64 = candidate.strip_prefix("v1,").unwrap_or(candidate);
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
      if constant_time_eq(&bytes, &expected) {
        return Ok(());
      }
    }
  }
  Err(OpenAiClientError::WebhookVerificationFailed("no signature candidate matched".into()))
}

/// Verify, then parse: the one-call path for webhook receivers.
pub fn parse_verified_webhook(args: &VerifyWebhookArgs<'_>) -> Result<WebhookEvent, OpenAiError> {
  verify_webhook_signature(args)?;
  parse_webhook_event(args.body).map_err(|e| {
    OpenAiClientError::WebhookVerificationFailed(format!("verified but unparseable event: {e}")).into()
  })
}

/// The inputs a sender signs (named fields — same transposition-hazard rationale as
/// [`VerifyWebhookArgs`]).
#[derive(Debug)]
pub struct SignWebhookArgs<'a> {
  pub secret: &'a OpenAiWebhookSecret,
  pub webhook_id: &'a str,
  /// Unix seconds, as it will appear in the `webhook-timestamp` header.
  pub webhook_timestamp: &'a str,
  pub body: &'a str,
}

/// Produce the `webhook-signature` header value (`v1,<base64(...)>`) for a payload — what
/// OpenAI's sender computes. Exists so tests (ours and consumers') can forge valid
/// deliveries against a known secret.
pub fn sign_webhook_payload(args: &SignWebhookArgs<'_>) -> String {
  let signed_content = format!("{}.{}.{}", args.webhook_id, args.webhook_timestamp, args.body);
  let mac = hmac_sha256(&args.secret.key, signed_content.as_bytes());
  format!("v1,{}", base64::engine::general_purpose::STANDARD.encode(mac))
}

/// HMAC-SHA256 (RFC 2104) built on the sha2 we already ship — no extra dependency.
pub(crate) fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
  const BLOCK: usize = 64;
  let mut key_block = [0u8; BLOCK];
  if key.len() > BLOCK {
    key_block[..32].copy_from_slice(&Sha256::digest(key));
  } else {
    key_block[..key.len()].copy_from_slice(key);
  }
  let mut inner = Sha256::new();
  let ipad: Vec<u8> = key_block.iter().map(|b| b ^ 0x36).collect();
  inner.update(&ipad);
  inner.update(message);
  let inner_hash = inner.finalize();

  let mut outer = Sha256::new();
  let opad: Vec<u8> = key_block.iter().map(|b| b ^ 0x5c).collect();
  outer.update(&opad);
  outer.update(inner_hash);
  outer.finalize().into()
}

/// Length-check + XOR-fold compare — no early exit on mismatch position.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
  if a.len() != b.len() {
    return false;
  }
  a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
  use super::*;

  /// RFC 4231 test case 2 — pins our hand-rolled HMAC to the standard.
  #[test]
  fn hmac_sha256_matches_rfc_4231() {
    let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
    assert_eq!(
      hex(&mac),
      "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
  }

  fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
  }

  fn secret() -> OpenAiWebhookSecret {
    OpenAiWebhookSecret::new("whsec_c2VjcmV0LWtleQ==").unwrap() // key = "secret-key"
  }

  fn sign(id: &str, ts: &str, body: &str) -> String {
    let mac = hmac_sha256(b"secret-key", format!("{id}.{ts}.{body}").as_bytes());
    format!("v1,{}", base64::engine::general_purpose::STANDARD.encode(mac))
  }

  const BODY: &str =
    r#"{ "id": "evt_1", "type": "response.completed", "created_at": 5, "data": { "id": "resp_ok" } }"#;

  #[test]
  fn valid_signature_verifies_and_parses() {
    let s = secret();
    let signature = sign("msg_1", "1750000000", BODY);
    let args = VerifyWebhookArgs {
      secret: &s,
      webhook_id: "msg_1",
      webhook_timestamp: "1750000000",
      webhook_signature: &signature,
      body: BODY,
    };
    let event = parse_verified_webhook(&args).unwrap();
    assert_eq!(event.response_id().unwrap().as_str(), "resp_ok");
  }

  #[test]
  fn rotated_secret_lists_are_accepted() {
    let s = secret();
    let good = sign("msg_1", "1750000000", BODY);
    let signature = format!("v1,AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA= {good}");
    let args = VerifyWebhookArgs {
      secret: &s,
      webhook_id: "msg_1",
      webhook_timestamp: "1750000000",
      webhook_signature: &signature,
      body: BODY,
    };
    assert!(verify_webhook_signature(&args).is_ok());
  }

  /// The Standard Webhooks spec's published cross-implementation vector — if this passes,
  /// our verifier agrees byte-for-byte with the ecosystem (and thus with OpenAI's sender).
  #[test]
  fn standard_webhooks_reference_vector_verifies() {
    let secret = OpenAiWebhookSecret::new("whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw").unwrap();
    let args = VerifyWebhookArgs {
      secret: &secret,
      webhook_id: "msg_p5jXN8AQM9LWM0D4loKWxJek",
      webhook_timestamp: "1614265330",
      webhook_signature: "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=",
      body: r#"{"test": 2432232314}"#,
    };
    assert!(verify_webhook_signature(&args).is_ok(), "reference vector must verify");
  }

  /// The exact payload shape from OpenAI's webhooks guide, end to end: sign it the way
  /// their sender would, then verify + parse + route.
  #[test]
  fn openai_docs_payload_end_to_end() {
    let docs_body = r#"{
      "object": "event",
      "id": "evt_685c059ae3a481909bdc86819b066fb6",
      "type": "response.completed",
      "created_at": 1750287018,
      "data": { "id": "resp_abc123" }
    }"#;
    let s = secret();
    let signature = sign("wh_685c059ae3a48190", "1750287018", docs_body);
    let args = VerifyWebhookArgs {
      secret: &s,
      webhook_id: "wh_685c059ae3a48190",
      webhook_timestamp: "1750287018",
      webhook_signature: &signature,
      body: docs_body,
    };
    let event = parse_verified_webhook(&args).unwrap();
    assert_eq!(event.event_id, "evt_685c059ae3a481909bdc86819b066fb6");
    assert_eq!(event.kind, crate::api::webhooks::webhook_event::WebhookEventKind::ResponseCompleted);
    assert_eq!(event.response_id().unwrap().as_str(), "resp_abc123");
    assert_eq!(event.created_at, 1750287018);
  }

  /// Failure events route too — the receiver needs them to mark jobs failed.
  #[test]
  fn failure_event_payloads_parse() {
    for (kind_wire, expected) in [
      ("response.failed", crate::api::webhooks::webhook_event::WebhookEventKind::ResponseFailed),
      ("response.cancelled", crate::api::webhooks::webhook_event::WebhookEventKind::ResponseCancelled),
      ("response.incomplete", crate::api::webhooks::webhook_event::WebhookEventKind::ResponseIncomplete),
    ] {
      let body = format!(
        r#"{{ "object": "event", "id": "evt_x", "type": "{kind_wire}", "created_at": 1, "data": {{ "id": "resp_sad" }} }}"#
      );
      let s = secret();
      let signature = sign("msg_f", "1700000000", &body);
      let args = VerifyWebhookArgs {
        secret: &s,
        webhook_id: "msg_f",
        webhook_timestamp: "1700000000",
        webhook_signature: &signature,
        body: &body,
      };
      let event = parse_verified_webhook(&args).unwrap();
      assert_eq!(event.kind, expected);
      assert_eq!(event.response_id().unwrap().as_str(), "resp_sad");
    }
  }

  /// Wrong-secret verification must fail even for an otherwise perfect payload.
  #[test]
  fn wrong_secret_rejected() {
    let right = secret();
    let wrong = OpenAiWebhookSecret::new("whsec_d3Jvbmcta2V5LXdyb25nLWtleQ==").unwrap();
    let signature = sign("msg_1", "1750000000", BODY);
    let args = VerifyWebhookArgs {
      secret: &wrong,
      webhook_id: "msg_1",
      webhook_timestamp: "1750000000",
      webhook_signature: &signature,
      body: BODY,
    };
    assert!(verify_webhook_signature(&args).is_err());
    let args_ok = VerifyWebhookArgs { secret: &right, ..args };
    assert!(verify_webhook_signature(&args_ok).is_ok());
  }

  #[test]
  fn tampered_body_id_or_timestamp_is_rejected() {
    let s = secret();
    let signature = sign("msg_1", "1750000000", BODY);
    for (id, ts, body) in [
      ("msg_1", "1750000000", r#"{"id":"evt_evil"}"#), // body swapped
      ("msg_2", "1750000000", BODY),                     // id swapped
      ("msg_1", "1750009999", BODY),                     // timestamp swapped
    ] {
      let args = VerifyWebhookArgs {
        secret: &s,
        webhook_id: id,
        webhook_timestamp: ts,
        webhook_signature: &signature,
        body,
      };
      assert!(verify_webhook_signature(&args).is_err(), "should reject ({id}, {ts})");
    }
  }
}
