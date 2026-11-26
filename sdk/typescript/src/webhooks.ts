import { isRequestId, object } from "./http.js";
import type { VerifyWebhookOptions, WebhookEvent } from "./types.js";

export class WebhookVerificationError extends Error {
  constructor(readonly code: "headers" | "timestamp" | "secret" | "signature" | "payload", message: string) {
    super(message);
    this.name = "WebhookVerificationError";
  }
}

const eventTypes = new Set(["request.succeeded", "request.canceled", "request.failed"]);
const bytes = new TextEncoder();
const utf8 = new TextDecoder("utf-8", { fatal: true });

export async function verifyWebhook(
  body: Uint8Array,
  headers: HeadersInit,
  signingSecrets: string | readonly string[],
  options: VerifyWebhookOptions = {},
): Promise<WebhookEvent> {
  const incoming = new Headers(headers);
  const id = incoming.get("webhook-id");
  const timestamp = incoming.get("webhook-timestamp");
  const signature = incoming.get("webhook-signature");
  if (!id || !/^whd_[0-9a-hjkmnp-tv-z]{26}$/.test(id) || !timestamp || !/^[0-9]{1,12}$/.test(timestamp)
      || !signature || signature.length > 1024) {
    throw new WebhookVerificationError("headers", "A required webhook header is invalid.");
  }

  const now = options.now ?? Math.floor(Date.now() / 1000);
  const tolerance = options.toleranceSeconds ?? 300;
  if (!Number.isSafeInteger(now) || !Number.isSafeInteger(tolerance) || tolerance < 0) {
    throw new RangeError("Webhook timestamp options must be whole seconds.");
  }
  if (Math.abs(now - Number(timestamp)) > tolerance) {
    throw new WebhookVerificationError("timestamp", "The webhook timestamp is outside the allowed window.");
  }
  if (body.byteLength > 1_048_576) {
    throw new WebhookVerificationError("payload", "The webhook body is too large.");
  }

  const signatures = signature.split(/\s+/).flatMap((part) => {
    const candidate = part.startsWith("v1,") ? decodeSha256(part.slice(3)) : undefined;
    return candidate ? [candidate] : [];
  });
  if (!signatures.length) throw new WebhookVerificationError("signature", "A valid v1 signature is required.");

  const secrets = typeof signingSecrets === "string" ? [signingSecrets] : signingSecrets;
  if (!Array.isArray(secrets) || !secrets.length || secrets.length > 4) {
    throw new WebhookVerificationError("secret", "Supply one to four TokenCraft signing secrets.");
  }
  const keys = secrets.map((secret) => {
    const raw = typeof secret === "string" && secret.startsWith("whsec_") ? decodeSha256(secret.slice(6)) : undefined;
    if (!raw) throw new WebhookVerificationError("secret", "A TokenCraft signing secret is invalid.");
    return raw;
  });

  const prefix = bytes.encode(`${id}.${timestamp}.`);
  const message = new Uint8Array(prefix.length + body.byteLength);
  message.set(prefix);
  message.set(body, prefix.length);
  const matches = await Promise.all(keys.map(async (raw) => {
    const key = await crypto.subtle.importKey("raw", raw, { name: "HMAC", hash: "SHA-256" }, false, ["verify"]);
    const checked = await Promise.all(signatures.map((candidate) => crypto.subtle.verify("HMAC", key, candidate, message)));
    return checked.some(Boolean);
  }));
  if (!matches.some(Boolean)) {
    throw new WebhookVerificationError("signature", "No signature matched the webhook body.");
  }

  let event: unknown;
  try {
    event = JSON.parse(utf8.decode(message.subarray(prefix.length)));
  } catch {
    throw new WebhookVerificationError("payload", "The signed body is not UTF-8 JSON.");
  }
  if (!object(event) || event.id !== id || event.schema_version !== 1
      || typeof event.type !== "string" || !eventTypes.has(event.type)
      || typeof event.occurred_at !== "string" || !Number.isFinite(Date.parse(event.occurred_at))
      || typeof event.sequence !== "number" || !Number.isSafeInteger(event.sequence) || event.sequence < 1
      || !object(event.data) || !isRequestId(event.data.request_id)) {
    throw new WebhookVerificationError("payload", "The signed event does not match the TokenCraft v1 contract.");
  }
  return event as unknown as WebhookEvent;
}

function decodeSha256(value: string) {
  if (!/^[A-Za-z0-9+/]{43}=$/.test(value)) return undefined;
  const binary = atob(value);
  if (btoa(binary) !== value) return undefined;
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}
