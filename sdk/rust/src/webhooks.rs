use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use reqwest::header::HeaderMap;
use serde::Deserialize;
use sha2::Sha256;

use crate::valid_id;

#[derive(Clone, Copy, Debug)]
pub struct WebhookOptions {
    pub now_seconds: Option<u64>,
    pub tolerance: Duration,
}

impl Default for WebhookOptions {
    fn default() -> Self {
        Self {
            now_seconds: None,
            tolerance: Duration::from_secs(300),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerificationError {
    #[error("a required webhook header is invalid")]
    Headers,
    #[error("the webhook timestamp is outside the allowed window")]
    Timestamp,
    #[error("a TokenCraft webhook signing secret is invalid")]
    Secret,
    #[error("no signature matched the webhook body")]
    Signature,
    #[error("the signed event does not match the TokenCraft v1 contract")]
    Payload,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum WebhookKind {
    #[serde(rename = "request.succeeded")]
    Succeeded,
    #[serde(rename = "request.canceled")]
    Canceled,
    #[serde(rename = "request.failed")]
    Failed,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WebhookData {
    pub request_id: String,
    #[serde(flatten)]
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VerifiedWebhook {
    pub id: String,
    pub schema_version: u32,
    #[serde(rename = "type")]
    pub kind: WebhookKind,
    pub occurred_at: String,
    pub sequence: u32,
    pub data: WebhookData,
}

/// Check the original received bytes before deserializing them into a v1 event.
pub fn verify_webhook(
    headers: &HeaderMap,
    body: &[u8],
    signing_secrets: &[&str],
    options: WebhookOptions,
) -> Result<VerifiedWebhook, VerificationError> {
    let id = single_header(headers, "webhook-id")?;
    let timestamp = single_header(headers, "webhook-timestamp")?;
    let signature = single_header(headers, "webhook-signature")?;
    if !valid_id(id, "whd_")
        || timestamp.len() > 12
        || !timestamp.bytes().all(|byte| byte.is_ascii_digit())
        || signature.len() > 1024
    {
        return Err(VerificationError::Headers);
    }
    let sent: u64 = timestamp.parse().map_err(|_| VerificationError::Headers)?;
    let now = match options.now_seconds {
        Some(now) => now,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| VerificationError::Timestamp)?
            .as_secs(),
    };
    if now.abs_diff(sent) > options.tolerance.as_secs() {
        return Err(VerificationError::Timestamp);
    }
    if body.len() > 1_048_576 {
        return Err(VerificationError::Payload);
    }
    if !(1..=4).contains(&signing_secrets.len()) {
        return Err(VerificationError::Secret);
    }
    let keys = signing_secrets
        .iter()
        .map(|secret| {
            secret
                .strip_prefix("whsec_")
                .and_then(decode_sha256)
                .ok_or(VerificationError::Secret)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let signatures: Vec<_> = signature
        .split_ascii_whitespace()
        .filter_map(|token| token.strip_prefix("v1,").and_then(decode_sha256))
        .collect();

    let prefix = format!("{id}.{timestamp}.");
    let verified = keys.iter().any(|key| {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts a 32-byte key");
        mac.update(prefix.as_bytes());
        mac.update(body);
        signatures
            .iter()
            .any(|candidate| mac.clone().verify_slice(candidate).is_ok())
    });
    if !verified {
        return Err(VerificationError::Signature);
    }

    let event: VerifiedWebhook =
        serde_json::from_slice(body).map_err(|_| VerificationError::Payload)?;
    if event.id != id
        || event.schema_version != 1
        || event.sequence == 0
        || event.occurred_at.is_empty()
        || !valid_id(&event.data.request_id, "req_")
    {
        return Err(VerificationError::Payload);
    }
    Ok(event)
}

fn single_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, VerificationError> {
    let values = headers.get_all(name);
    let mut values = values.iter();
    values
        .next()
        .filter(|_| values.next().is_none())
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .ok_or(VerificationError::Headers)
}

fn decode_sha256(raw: &str) -> Option<[u8; 32]> {
    if raw.len() != 44 {
        return None;
    }
    STANDARD.decode(raw).ok()?.try_into().ok()
}
