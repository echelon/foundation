import type { ProviderAttempt, ProviderPath, RequestStatus, TraceCorrelation } from "./types.js";

export function validateTraceparent(value: string): string {
  const invalid = () => new TypeError("Supply a valid W3C traceparent.");
  if (typeof value !== "string" || value.length > 256 || /[^\x21-\x7e]|,/.test(value)) throw invalid();
  const base = /^([0-9a-f]{2})-([0-9a-f]{32})-([0-9a-f]{16})-([0-9a-f]{2})/.exec(value);
  if (!base) throw invalid();
  const [, version, trace, parent] = base;
  const suffix = value.slice(55);
  if (version === "ff" || trace === "00000000000000000000000000000000" || parent === "0000000000000000"
      || (suffix && (version === "00" || !suffix.startsWith("-")))) throw invalid();
  return value;
}

export type TraceTone = "ok" | "active" | "warning" | "error" | "neutral";

export interface ComputeTargetAttempt {
  step_id: string;
  selection: {
    home_cell: string;
    stack: string;
    target: string;
    cloud: string;
    region: string;
    zone: string;
    priority: number;
  };
}

export interface TraceStep {
  id: string;
  parent_id: string | null;
  name: string;
  component: string;
  kind: "point" | "interval";
  started_at: string;
  ended_at: string | null;
  status: string;
  tone: TraceTone;
  logs: { at: string; code: string; message: string; severity: TraceTone }[];
  attributes: { label: string; value: string }[];
}

export interface TraceSummary {
  headline: string;
  detail?: string;
  next_check: { label: string; step_id: string };
  signals: { label: string; value: string; tone: TraceTone; step_id: string }[];
}

export interface TraceEvidence {
  schema_version: number;
  request_id: string;
  status: RequestStatus;
  subscription_configured: boolean;
  delivery_status: "pending" | "sending" | "delivered" | "dead" | null;
  delivery_attempts_omitted: number;
  steps: TraceStep[];
  coverage: string[];
  summary?: TraceSummary;
  routing?: Pick<ProviderPath, "admitted_offer_id" | "downstream"> & {
    attempts: Omit<ProviderAttempt, "adapter" | "correlation_id" | "compute_attempts">[];
  };
}

export interface RequestTraceSnapshot {
  revision: string;
  observed_at: string;
  analysis_available: boolean;
  evidence: TraceEvidence;
  provider_path?: ProviderPath;
  correlation?: TraceCorrelation;
}

/** OTLP/HTTP JSON ExportTraceServiceRequest. */
export interface OtlpTrace {
  resourceSpans: Record<string, unknown>[];
}

export interface TraceBaseline {
  model: string;
  provider: string;
  offer_id: string;
  window_start: string;
  window_end: string;
  request_ids: string[];
  timings: {
    metric: "elapsed" | "queue" | "publication";
    request_ms: number | null;
    median_ms: number | null;
    samples: number;
  }[];
}
