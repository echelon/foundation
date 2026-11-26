import { TokenCraftError } from "./http.js";
import type {
  ArtifactRef, AudioProviderResult, CallOptions, ImageProviderResult, MeshProviderResult,
  ProviderResult, VideoProviderResult,
} from "./types.js";

export type ContentPhase = "processing" | "available" | "archiving" | "archived" | "removed" | "expired";

export interface RestoreProgress {
  state: "pending" | "ready" | "failed";
  ready_objects: number;
  total_objects: number;
  requested_at: string;
  retry_at: string;
}

export interface ContentMetadata {
  phase: ContentPhase;
  media?: boolean;
  archive_at: string | null;
  expires_at: string | null;
  restore?: RestoreProgress;
}

export type RetainedResult = ProviderResult["result"] | ImageProviderResult["result"] | VideoProviderResult["result"]
  | AudioProviderResult["result"] | MeshProviderResult["result"];

export interface RetainedContent extends ContentMetadata {
  /** The original chat or connector input as recorded by the gateway. */
  input?: Record<string, unknown>;
  result?: RetainedResult;
}

export interface DownloadOptions extends CallOptions {
  location?: "warm" | "archived";
}

export interface VerifiedArtifact {
  contentType: string;
  bytes: Uint8Array;
}

const maxArtifactBytes = 8 * 1024 * 1024;
const mediaTypes = new Set(["image/png", "video/mp4", "audio/wav", "model/gltf-binary"]);

export function validateArtifactRef(artifact: ArtifactRef) {
  if (!Number.isInteger(artifact?.slot) || artifact.slot < 0 || artifact.slot > 7
      || !Number.isInteger(artifact.byte_length) || artifact.byte_length < 1 || artifact.byte_length > maxArtifactBytes
      || typeof artifact.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(artifact.sha256)) {
    throw new TypeError("Supply a valid saved artifact reference.");
  }
}

export async function readArtifact(response: Response, artifact: ArtifactRef, requestId?: string): Promise<VerifiedArtifact> {
  const invalid = () => new TokenCraftError(response.status, "invalid_artifact", "The gateway artifact did not match its saved reference.", requestId);
  const contentType = response.headers.get("content-type");
  const contentLength = response.headers.get("content-length");
  if (response.status !== 200 || !contentType || !mediaTypes.has(contentType) || !response.body
      || response.headers.get("x-tokencraft-content-sha256") !== artifact.sha256
      || (contentLength !== null && contentLength !== String(artifact.byte_length))) {
    void response.body?.cancel().catch(() => undefined);
    throw invalid();
  }

  const bytes = new Uint8Array(artifact.byte_length);
  const reader = response.body.getReader();
  let length = 0;
  try {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      if (value.byteLength > bytes.length - length) {
        void reader.cancel().catch(() => undefined);
        throw invalid();
      }
      bytes.set(value, length);
      length += value.byteLength;
    }
  } finally {
    reader.releaseLock();
  }
  if (length !== bytes.length) throw invalid();
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  const checksum = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
  if (checksum !== artifact.sha256) throw invalid();
  return { contentType, bytes };
}
