/**
 * Per-model catalog usage stats — the frontend stand-in for the design's HOT
 * catalog data (`model_rankings_and_reports.md` §3.3 + §5 `model_usage_summary`,
 * served as `catalog_stats.json`). In production this is one CDN-cached payload
 * the catalog index sorts client-side; here the numbers are derived
 * deterministically from the slug so they're stable across reloads.
 */

import { type Model, type Price } from "./catalog";

function hashFloat(seed: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return ((h >>> 0) % 100000) / 100000;
}

export type ModelStats = {
  /** Successful generations in the trailing window (volume by count). */
  generations: number;
  /** Output units in the window (images, video-seconds, tokens, …). */
  outputUnits: number;
  /** List-price-weighted volume in USD (Σ output × list unit price). PUBLIC, not revenue. */
  listValue: number;
  /** Headline list unit price (for price sorts). */
  price: number;
  priceUnit: Price["unit"];
  /** Week-over-week generation growth ratio (e.g. +0.18). */
  wow: number;
  /** ~14 daily generation counts, oldest → newest. */
  sparkline: number[];
};

/** Per-modality output-units-per-generation and price normalization to per-output-unit. */
function outputPerGen(model: Model): number {
  switch (model.modality) {
    case "video":
      return 6; // seconds
    case "chat":
      return 2500; // tokens
    default:
      return 1; // images, meshes, worlds
  }
}
function perUnitPrice(p: Price): number {
  if (p.unit === "per_million_tokens_in" || p.unit === "per_million_tokens_out") return p.usd / 1_000_000;
  return p.usd;
}

export function statsFor(model: Model): ModelStats {
  // intrinsic popularity weight, skewed so a few models dominate
  const w = Math.pow(hashFloat(`${model.slug}|pop`), 2.2);
  const generations = Math.round(2_000 + w * 4_800_000);
  const outputUnits = Math.round(generations * outputPerGen(model));
  const price = model.pricing[0].usd;
  const listValue = outputUnits * perUnitPrice(model.pricing[0]);
  const wow = Number(((hashFloat(`${model.slug}|wow`) - 0.4) * 0.8).toFixed(3));

  const sparkline: number[] = [];
  let v = 30 + hashFloat(`${model.slug}|s0`) * 50;
  const drift = (hashFloat(`${model.slug}|drift`) - 0.42) * 5;
  for (let i = 0; i < 14; i++) {
    v = Math.max(5, v + drift + (hashFloat(`${model.slug}|d${i}`) - 0.5) * 12);
    sparkline.push(Math.round(v));
  }

  return { generations, outputUnits, listValue, price, priceUnit: model.pricing[0].unit, wow, sparkline };
}

/* ── Sort options (mirror the design's §3.3 sort keys) ── */
export type SortKey = "generations" | "output" | "value" | "price_desc" | "price_asc" | "trending" | "newest";
export const SORTS: { key: SortKey; label: string }[] = [
  { key: "generations", label: "Volume (generations)" },
  { key: "output", label: "Volume (output)" },
  { key: "value", label: "Volume (price-weighted)" },
  { key: "price_desc", label: "Cost (high → low)" },
  { key: "price_asc", label: "Cost (low → high)" },
  { key: "trending", label: "Trending (WoW)" },
  { key: "newest", label: "Newest" },
];

export type RankedModel = { model: Model; stats: ModelStats; rank: number };

/** Rank a set of models by a sort key. Rank = position in the current order. */
export function rankModels(models: Model[], sort: SortKey): RankedModel[] {
  const withStats = models.map((model) => ({ model, stats: statsFor(model) }));
  withStats.sort((a, b) => {
    switch (sort) {
      case "generations":
        return b.stats.generations - a.stats.generations;
      case "output":
        return b.stats.outputUnits - a.stats.outputUnits;
      case "value":
        return b.stats.listValue - a.stats.listValue;
      case "price_asc":
        return a.stats.price - b.stats.price;
      case "price_desc":
        return b.stats.price - a.stats.price;
      case "trending":
        return b.stats.wow - a.stats.wow;
      case "newest":
        return b.model.released.localeCompare(a.model.released);
    }
  });
  return withStats.map((x, i) => ({ ...x, rank: i + 1 }));
}

/** The value + label to show for the active sort (the "sorted metric" column). */
export function sortMetric(rm: RankedModel, sort: SortKey): { label: string; value: string } {
  const s = rm.stats;
  switch (sort) {
    case "generations":
      return { label: "Generations", value: compact(s.generations) };
    case "output":
      return { label: "Output", value: `${compact(s.outputUnits)} ${outputUnitShort(rm.model.modality)}` };
    case "value":
      return { label: "Value", value: `$${compact(s.listValue)}` };
    case "price_asc":
    case "price_desc":
      return { label: "Price", value: priceShort(rm.model.pricing[0]) };
    case "trending":
      return { label: "Weekly", value: `${s.wow >= 0 ? "+" : ""}${(s.wow * 100).toFixed(0)}%` };
    case "newest":
      return { label: "Released", value: rm.model.released };
  }
}

function outputUnitShort(modality: Model["modality"]): string {
  switch (modality) {
    case "video":
      return "sec";
    case "chat":
      return "tok";
    case "3d":
      return "meshes";
    case "world":
      return "worlds";
    default:
      return "imgs";
  }
}
function priceShort(p: Price): string {
  const money = `$${p.usd < 0.1 ? p.usd.toFixed(3) : p.usd.toFixed(2)}`;
  const unit: Record<Price["unit"], string> = {
    per_image: "/img",
    per_video_second: "/sec",
    per_mesh: "/mesh",
    per_splat: "/world",
    per_million_tokens_in: "/Mtok in",
    per_million_tokens_out: "/Mtok out",
  };
  return money + unit[p.unit];
}
export function compact(n: number): string {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)}B`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}K`;
  return String(Math.round(n));
}
