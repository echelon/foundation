import { gatewayRequest, withProject, type Provider } from "@/lib/gateway";
import type { Offer as CatalogOffer } from "@/surfaces/comparison/api";

export type ProviderScope = { account_id: string; project_id?: string };

export type Kind = "text" | "image" | "video" | "audio" | "mesh";
export const kindLabel: Record<Kind, string> = {
  text: "Chat", image: "Image", video: "Video", audio: "Audio", mesh: "3D",
};

export type Pricing =
  | { kind: "text"; usd_nanos_per_1000_input_tokens: number; usd_nanos_per_1000_output_tokens: number }
  | { kind: "image"; usd_nanos_per_image: number; aspect_ratios: string[] }
  | { kind: "video"; usd_nanos_per_second: number; durations_seconds: number[]; aspect_ratios: string[]; resolution: string }
  | { kind: "audio"; usd_nanos_per_second: number; durations_seconds: number[]; format: "wav" }
  | { kind: "mesh"; usd_nanos_per_mesh: number; format: "glb" };

export type Offer = {
  id: string;
  model: string;
  provider: Provider;
  adapter: CatalogOffer["adapter"];
  binding: string | null;
  pricing: Pricing;
  reviewed_pricing: Pricing;
  allowed_by_policy: boolean;
  connection_id: string;
  selected_projects: string[] | null;
  available_in_project: boolean;
  excluded_in_project: boolean;
};

export type Inventory = {
  base_revision: string;
  active_revision: string;
  catalog_revision: string;
  offers: Offer[];
  policy_revision: number;
  discovery: boolean;
  provider_key_management: boolean;
  credential_bindings?: string[];
};

export type Binding =
  | { kind: "connector"; name: string; origin: string; revision: string; token_env: string }
  | { kind: "chat"; name: string; endpoint: string; key_env?: string };

type ModelInput = { id: string; model: string; upstream_model: string; pricing: Pricing };
export type Change =
  | { kind: "attach"; name: string; provider_id?: Provider; binding: Binding; models: ModelInput[] }
  | { kind: "add_models"; binding_name: string; models: ModelInput[] }
  | { kind: "update_price"; offer_id: string; primary_usd_nanos: number; output_usd_nanos?: number };

export type Preview = {
  project_id?: string;
  catalog_patch?: boolean;
  base_revision: string;
  base_catalog_revision: string;
  proposed_revision: string;
  provider_name: string | null;
  catalog: { version: 1; offers: { id: string; pricing: Pricing }[] };
  binding: Binding | null;
  changed_offer_ids: string[];
  rate_feed_active: boolean;
};

export type DiscoveredModel = {
  model_id: string;
  name: string;
  kind: Kind;
  source_url: string;
  price: { published: string; source_url: string; amount_usd: string | null; output_usd: string | null } | null;
};

export type Discovery = { checked_at: string; models: DiscoveredModel[] };
export type Connection = Binding["kind"];

export type RateFields = { amount: string; output: string };

export type ModelDraft = RateFields & {
  key: string;
  upstream: string;
  alias: string;
  kind: Kind;
  ratios: string;
  durations: string;
  resolution: string;
  reviewed: boolean;
  evidence?: DiscoveredModel & { checked_at: string };
};

export function newModel(kind: Kind): ModelDraft {
  return {
    key: crypto.randomUUID(), kind, upstream: "", alias: "", amount: "", output: "",
    ratios: "", durations: "", resolution: "", reviewed: false,
  };
}

export function slug(value: string) {
  return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "").slice(0, 64);
}

export function defaultEnv(binding: string, connection: Connection) {
  const suffix = binding.toUpperCase().replace(/[^A-Z0-9]+/g, "_");
  if (!suffix) return "";
  return connection === "chat" ? `TOKENCRAFT_LOCAL_CHAT_KEY_${suffix}` : `TOKENCRAFT_CONNECTOR_TOKEN_${suffix}`;
}

// The browser displays published USD units. The gateway stores integer nano-USD per
// runtime unit, including per 1,000 tokens. Excess precision is rejected.
export function usdNanos(value: string, kind: Kind) {
  const match = /^(\d{1,10})(?:\.(\d{1,24}))?$/.exec(value.trim());
  if (!match) throw new Error("Enter an exact USD amount, including 0 for an explicitly free rate.");
  const places = kind === "text" ? 6 : 9;
  const fraction = (match[2] ?? "").replace(/0+$/, "");
  if (fraction.length > places) throw new Error("This price is more precise than the gateway can represent.");
  const nanos = BigInt(match[1]) * 10n ** BigInt(places) + BigInt(fraction.padEnd(places, "0"));
  if (nanos > 1_000_000_000n) throw new Error("This price exceeds the gateway’s per-unit limit.");
  return Number(nanos);
}

function inputUsd(nanos: number, kind: Kind) {
  const places = kind === "text" ? 6 : 9;
  const digits = String(nanos).padStart(places + 1, "0");
  return `${digits.slice(0, -places)}.${digits.slice(-places)}`.replace(/\.?0+$/, "");
}

export function readRates(price: Pricing): RateFields {
  switch (price.kind) {
    case "text": return { amount: inputUsd(price.usd_nanos_per_1000_input_tokens, "text"), output: inputUsd(price.usd_nanos_per_1000_output_tokens, "text") };
    case "image": return { amount: inputUsd(price.usd_nanos_per_image, "image"), output: "" };
    case "video":
    case "audio": return { amount: inputUsd(price.usd_nanos_per_second, price.kind), output: "" };
    case "mesh": return { amount: inputUsd(price.usd_nanos_per_mesh, "mesh"), output: "" };
  }
}

export function rateUnit(kind: Kind) {
  switch (kind) {
    case "text": return "1M tokens";
    case "image": return "image";
    case "video":
    case "audio": return "second";
    case "mesh": return "asset";
  }
}

function pricing(model: ModelDraft): Pricing {
  const amount = usdNanos(model.amount, model.kind);
  const ratios = model.ratios.split(",").map((ratio) => ratio.trim()).filter(Boolean);
  const durations = () => {
    const entries = model.durations.split(",").map((duration) => duration.trim());
    if (entries.some((duration) => !/^\d{1,3}$/.test(duration))) {
      throw new Error("Enter supported durations as whole seconds separated by commas.");
    }
    return entries.map(Number);
  };
  switch (model.kind) {
    case "text": return {
      kind: "text", usd_nanos_per_1000_input_tokens: amount,
      usd_nanos_per_1000_output_tokens: usdNanos(model.output, "text"),
    };
    case "image": return { kind: "image", usd_nanos_per_image: amount, aspect_ratios: ratios };
    case "video": return {
      kind: "video", usd_nanos_per_second: amount, aspect_ratios: ratios,
      durations_seconds: durations(), resolution: model.resolution.trim(),
    };
    case "audio": return { kind: "audio", usd_nanos_per_second: amount, durations_seconds: durations(), format: "wav" };
    case "mesh": return { kind: "mesh", usd_nanos_per_mesh: amount, format: "glb" };
  }
}

export function buildModels(rows: ModelDraft[], binding: string): ModelInput[] {
  return rows.map((row, index) => {
    try {
      const upstream = row.upstream.trim();
      const alias = row.alias.trim() || slug(upstream);
      if (!upstream || !alias) throw new Error("Enter the model ID and a gateway alias.");
      if (row.evidence && !row.reviewed) throw new Error("Review the suggested fields against the source.");
      const suffix = slug(alias).slice(0, 29);
      const prefix = slug(binding).slice(0, 34);
      if (!prefix || !suffix) throw new Error("The binding name and alias need letters or numbers.");
      return { id: `${prefix}_${suffix}`, model: alias, upstream_model: upstream, pricing: pricing(row) };
    } catch (failure) {
      throw new Error(`Model ${index + 1} — ${failure instanceof Error ? failure.message : "check the model fields."}`);
    }
  });
}

function path(scope: ProviderScope, tail = "") {
  const base = scope.project_id ? `/v1/projects/${encodeURIComponent(scope.project_id)}/providers` : "/v1/providers";
  return withProject(base + tail, undefined, scope.account_id);
}

function post<T>(url: string, data: unknown, signal: AbortSignal) {
  return gatewayRequest<T>(url, {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(data), signal,
  });
}

export const providerOnboarding = {
  inventory: (scope: ProviderScope, signal: AbortSignal) => gatewayRequest<Inventory>(path(scope), { signal }),
  discover: (scope: ProviderScope, source: string, signal: AbortSignal) =>
    post<Discovery>(path(scope, "/discover"), { source_url: source }, signal),
  preview: (scope: ProviderScope, baseRevision: string, change: Change, projects: string[] | null, signal: AbortSignal) =>
    post<Preview>(path(scope), { base_revision: baseRevision, change, projects }, signal),
  access: (scope: ProviderScope, revision: number, connectionId: string,
    change: { kind: "share"; projects: string[] | null } | { kind: "exclude"; excluded: boolean }) =>
    gatewayRequest<{ revision: number }>(withProject("/v1/provider-access", scope.project_id, scope.account_id), {
      method: "PUT", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ connection_id: connectionId, expected_revision: revision, change }),
    }),
};
