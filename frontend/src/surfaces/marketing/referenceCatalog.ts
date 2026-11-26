import { downloadBlob } from "@/lib/download";
import { modalityNames, type Modality } from "@/surfaces/requests/modelOptions";
import { referencePlugin } from "./referencePlugins";

export const referenceCatalogUrl = "/catalog/artcraft-v1.json";
export const referenceCatalogFile = "artcraft-v1.json";

export type ReferenceOffer = {
  id: string;
  model: string;
  name: string;
  plugin: string;
  upstream_model: string;
  kind: Modality;
  preset: string;
  capabilities?: unknown;
  rates: { usd_nanos: number; unit: string; terms?: string }[];
  source_url: string;
};

export type ReferenceCatalog = { version: 1; revision: string; offers: ReferenceOffer[] };
export type ReferenceSnapshot = { catalog: ReferenceCatalog; bytes: ArrayBuffer; checksum?: string };

export async function loadReferenceSnapshot(signal: AbortSignal): Promise<ReferenceSnapshot> {
  const response = await fetch(referenceCatalogUrl, { credentials: "omit", cache: "no-cache", signal });
  if (!response.ok) throw new Error("Catalog unavailable");
  const bytes = await response.arrayBuffer();
  const catalog = parseReferenceCatalog(bytes);
  const hash = await globalThis.crypto?.subtle?.digest("SHA-256", bytes).catch(() => undefined);
  const checksum = hash && Array.from(new Uint8Array(hash), (byte) => byte.toString(16).padStart(2, "0")).join("");
  return { catalog, bytes, checksum };
}

export function downloadReferenceSnapshot(snapshot: ReferenceSnapshot) {
  downloadBlob(new Blob([snapshot.bytes], { type: "application/json" }), referenceCatalogFile);
}

export function parseReferenceCatalog(bytes: ArrayBuffer): ReferenceCatalog {
  const catalog = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as
    (ReferenceCatalog & { publisher?: string; currency?: string }) | null;
  if (catalog?.version !== 1 || catalog.publisher !== "ArtCraft" || catalog.currency !== "USD"
    || typeof catalog.revision !== "string" || !catalog.revision || !Array.isArray(catalog.offers)) {
    throw new Error("Unsupported catalog format");
  }

  const ids = new Set<string>();
  for (const offer of catalog.offers) {
    if (!offer || [offer.id, offer.model, offer.name, offer.plugin, offer.upstream_model, offer.preset, offer.source_url].some(
      (value) => typeof value !== "string" || !value,
    ) || ids.has(offer.id) || !Object.hasOwn(modalityNames, offer.kind) || !Array.isArray(offer.rates)
      || offer.rates.some((rate) => !rate || !Number.isSafeInteger(rate.usd_nanos) || rate.usd_nanos < 0
        || typeof rate.unit !== "string" || (rate.terms !== undefined && typeof rate.terms !== "string"))) {
      throw new Error("Invalid reference offer");
    }
    const source = new URL(offer.source_url);
    if (source.protocol !== "https:" || source.username || source.password) throw new Error("Invalid rate source");
    ids.add(offer.id);
  }
  return catalog;
}

export function canPrepare(offer: ReferenceOffer) {
  if (!offer.capabilities || typeof offer.capabilities !== "object" || Array.isArray(offer.capabilities)) return false;
  return referencePlugin(offer.plugin)?.kinds.includes(offer.kind) ?? false;
}
