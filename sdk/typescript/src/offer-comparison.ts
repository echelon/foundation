import type { TokenCraft } from "./client.js";
import { milliseconds, TokenCraftError, TokenCraftTimeoutError } from "./http.js";
import type {
  CallOptions, Candidate, CostState, Provider, Quote, QuoteInput, RequestStatus, RequestTrace, SubmitOptions, WaitOptions,
} from "./types.js";

const maxOffers = 4;
const maxSamples = 25;
const parallelRequests = 4;
const identifier = /^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$/;
const nanosPerUnit = 1_000_000_000n;

export interface OfferComparisonOptions {
  runId: string;
  input: QuoteInput;
  offerIds: readonly string[];
  /** Each arm receives these prompts in the same order. Use null for promptless fixtures. */
  prompts: readonly (string | null)[];
  maxEstimatedUsdNanos: number;
  clientRef?: string;
  appRef?: string;
}

export interface OfferComparisonPlan extends Omit<OfferComparisonOptions, "input"> {
  quotes: readonly Quote[];
  estimatedUsdNanos: number;
}

type ObservedCost = { state: Exclude<CostState, "estimated">; usdNanos: string };
const costPriority: Record<ObservedCost["state"], number> = {
  mock_reported: 0, provider_reported: 1, supplier_settled: 2,
};
type ProviderCharge = { provider: Provider; unit: string; quantity: string; observedAt: string };
type OutputShape =
  | { kind: "image"; count: number; aspectRatio?: string; sizes?: { width: number; height: number }[] }
  | { kind: "video"; durationMs: number; width?: number; height?: number; aspectRatio?: string; resolution?: string }
  | { kind: "audio"; durationMs: number; sampleRateHz?: number; channels?: number; format?: string }
  | { kind: "mesh"; meshCount?: number; primitiveCount?: number; format?: string }
  | { kind: "text"; inputTokens: number | null; outputTokens: number | null };

export interface OfferComparisonSample {
  sampleIndex: number;
  idempotencyKey: string;
  status: RequestStatus | "not_started" | "submit_error" | "observe_error";
  requestId?: string;
  estimateUsdNanos: number;
  cost?: ObservedCost;
  providerCharge?: ProviderCharge;
  elapsedMs?: number;
  output?: OutputShape;
  errorCode?: string;
  outputErrorCode?: string;
  refreshErrorCode?: string;
}

export interface OfferComparisonArm {
  offerId: string;
  quoteId: string;
  catalogRevision: string;
  routing?: Quote["routing"];
  estimatedUsdNanos: number;
  spentUsdNanos: Record<ObservedCost["state"], string>;
  providerCharges?: { provider: Provider; unit: string; quantity: string; observations: number }[];
  unknownCostSamples: number;
  statuses: Partial<Record<OfferComparisonSample["status"], number>>;
  medianCompletionMs?: number;
  samples: OfferComparisonSample[];
}

/** Get comparable pinned quotes without submitting any work. Retain the plan for an exact replay. */
export async function planOfferComparison(
  client: TokenCraft,
  options: OfferComparisonOptions,
  signal?: AbortSignal,
): Promise<OfferComparisonPlan> {
  const { input, ...settings } = options;
  const plan = { ...settings, offerIds: [...settings.offerIds], prompts: [...settings.prompts] };
  validateSettings(plan);
  const quotes = await Promise.all(plan.offerIds.map(offerId =>
    client.quote(input, { scope: "project", offerId, clientRef: plan.clientRef, signal }),
  ));
  return { ...plan, quotes, estimatedUsdNanos: validateQuotes(plan, quotes) };
}

/** Submit bounded parallel arms and return every observed request ID, including partial outcomes. */
export async function runOfferComparison(
  client: TokenCraft,
  plan: OfferComparisonPlan,
  options: WaitOptions = {},
): Promise<OfferComparisonArm[]> {
  if (options.timeoutMs !== undefined) milliseconds(options.timeoutMs, "timeoutMs");
  if (options.pollIntervalMs !== undefined) milliseconds(options.pollIntervalMs, "pollIntervalMs");
  const snapshot = validatedSnapshot(plan);
  await client.list({ clientRef: snapshot.clientRef, appRef: snapshot.appRef, signal: options.signal });

  const rows = snapshot.quotes.map((quote, arm) => snapshot.prompts.map((_, sampleIndex): OfferComparisonSample => ({
    sampleIndex,
    idempotencyKey: sampleKey(snapshot.runId, snapshot.offerIds[arm], sampleIndex),
    status: "not_started",
    estimateUsdNanos: selected(quote).estimated_usd_nanos,
  })));
  const jobs = snapshot.prompts.flatMap((prompt, sampleIndex) => snapshot.quotes.map((quote, arm) => ({
    quote, prompt, row: rows[arm][sampleIndex],
  })));
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(parallelRequests, jobs.length) }, async () => {
    while (!options.signal?.aborted && next < jobs.length) {
      const { quote, prompt, row } = jobs[next++];
      const slotReleased = await observe(client, quote, prompt, row, snapshot, options);
      if (!slotReleased) return;
    }
  }));
  return snapshot.quotes.map((quote, arm) => summarize(quote, snapshot.offerIds[arm], rows[arm]));
}

/** Refresh saved observations through request reads, including after the original quotes expire. */
export async function refreshOfferComparison(
  client: TokenCraft,
  plan: OfferComparisonPlan,
  report: readonly OfferComparisonArm[],
  { signal }: CallOptions = {},
): Promise<OfferComparisonArm[]> {
  const snapshot = validatedSnapshot(plan);
  const arms = structuredClone(report);
  if (arms.length !== snapshot.quotes.length) throw new TypeError("The report does not match the comparison plan.");

  const jobs = arms.flatMap((arm, index) => {
    const quote = snapshot.quotes[index];
    const offer = snapshot.offerIds[index];
    const estimated = selected(quote).estimated_usd_nanos;
    if (arm.offerId !== offer || arm.quoteId !== quote.id || arm.catalogRevision !== quote.catalog_revision
      || arm.samples.length !== snapshot.prompts.length
      || arm.samples.some((row, sample) => row.sampleIndex !== sample
        || row.idempotencyKey !== sampleKey(snapshot.runId, offer, sample) || row.estimateUsdNanos !== estimated)) {
      throw new TypeError("The report does not match the comparison plan.");
    }
    return arm.samples.flatMap(row => row.requestId ? [{ quote, row, requestId: row.requestId }] : []);
  });
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(parallelRequests, jobs.length) }, async () => {
    while (!signal?.aborted && next < jobs.length) {
      const { quote, row, requestId } = jobs[next++];
      try {
        const trace = await client.get(requestId, { signal });
        if (trace.id !== requestId || trace.quote.id !== quote.id
          || trace.quote.catalog_revision !== quote.catalog_revision
          || trace.quote.selected_offer_id !== quote.selected_offer_id
          || (snapshot.clientRef !== undefined && trace.client_ref !== snapshot.clientRef)
          || (snapshot.appRef !== undefined && trace.app_ref !== snapshot.appRef)) {
          row.refreshErrorCode = "trace_evidence_mismatch";
          continue;
        }
        await recordTrace(client, trace, row, signal);
        delete row.refreshErrorCode;
      } catch (error) {
        row.refreshErrorCode = observationError(error);
      }
    }
  }));
  return arms.map((arm, index) => summarize(snapshot.quotes[index], arm.offerId, arm.samples));
}

function sampleKey(runId: string, offerId: string, sampleIndex: number): string {
  return `comparison:${runId.length}:${runId}:${offerId}:${sampleIndex}`;
}

function validatedSnapshot(plan: OfferComparisonPlan): OfferComparisonPlan {
  const snapshot = structuredClone(plan);
  validateSettings(snapshot);
  if (validateQuotes(snapshot, snapshot.quotes) !== snapshot.estimatedUsdNanos) {
    throw new TypeError("The comparison estimate changed.");
  }
  return snapshot;
}

function validateSettings(plan: Omit<OfferComparisonOptions, "input">) {
  if (!/^[A-Za-z0-9._:-]{8,40}$/.test(plan.runId)) {
    throw new TypeError("Use an 8–40 character comparison run ID.");
  }
  if (plan.offerIds.length < 2 || plan.offerIds.length > maxOffers
    || new Set(plan.offerIds).size !== plan.offerIds.length || plan.offerIds.some(id => !identifier.test(id))) {
    throw new TypeError("Use two to four distinct gateway offer IDs.");
  }
  if (plan.prompts.length < 1 || plan.prompts.length > maxSamples
    || plan.prompts.some(prompt => prompt !== null && (typeof prompt !== "string" || !prompt.trim()))) {
    throw new TypeError("Use one to 25 samples with nonempty prompts or null fixture entries.");
  }
  if (!Number.isSafeInteger(plan.maxEstimatedUsdNanos) || plan.maxEstimatedUsdNanos < 0) {
    throw new TypeError("Supply a nonnegative total estimate cap in whole nano-USD.");
  }
}

function selected(quote: Quote): Candidate & { estimated_usd_nanos: number } {
  const row = quote.candidates.find(candidate => candidate.offer_id === quote.selected_offer_id);
  const amount = row?.estimated_usd_nanos;
  if (!row || row.reason !== "eligible" || amount === null || amount === undefined
    || !Number.isSafeInteger(amount) || amount < 0) {
    throw new Error(`Offer ${quote.pinned_offer_id ?? "pin"} is ineligible or lacks a valid estimate.`);
  }
  return { ...row, estimated_usd_nanos: amount };
}

function validateQuotes(plan: Omit<OfferComparisonOptions, "input">, quotes: readonly Quote[]): number {
  if (quotes.length !== plan.offerIds.length) throw new TypeError("The comparison is missing quotes.");
  let signature: string | undefined;
  let estimate = 0;
  for (const [index, quote] of quotes.entries()) {
    const offer = plan.offerIds[index];
    if (quote.pinned_offer_id !== offer || quote.selected_offer_id !== offer || quote.currency !== "USD") {
      throw new Error(`The gateway did not return an eligible exact quote for ${offer}.`);
    }
    const row = selected(quote);
    const key = JSON.stringify([
      quote.catalog_revision, quote.input, quote.routing?.organization_revision, quote.routing?.project_revision,
      row.applied_aspect_ratio, row.output_size,
    ]);
    if (signature !== undefined && signature !== key) {
      throw new Error("Comparison quotes must share a catalog, policy revision, workload, and quoted output shape.");
    }
    signature = key;
    estimate += row.estimated_usd_nanos * plan.prompts.length;
  }
  if (!Number.isSafeInteger(estimate) || estimate > plan.maxEstimatedUsdNanos) {
    throw new RangeError("The combined estimate exceeds the comparison cap.");
  }
  return estimate;
}

function promptOptions(kind: QuoteInput["workload"]["kind"], prompt: string | null): Partial<SubmitOptions> {
  if (prompt === null) return {};
  switch (kind) {
    case "image": return { imagePrompt: prompt };
    case "video": return { videoPrompt: prompt };
    case "audio": return { audioPrompt: prompt };
    case "mesh": return { meshPrompt: prompt };
    case "text": return { messages: [{ role: "user", content: prompt }] };
  }
}

async function observe(
  client: TokenCraft,
  quote: Quote,
  prompt: string | null,
  row: OfferComparisonSample,
  plan: OfferComparisonPlan,
  options: WaitOptions,
): Promise<boolean> {
  const { signal } = options;
  let trace: RequestTrace;
  try {
    const receipt = await client.submit(quote, {
      idempotencyKey: row.idempotencyKey, clientRef: plan.clientRef, appRef: plan.appRef, signal,
      ...promptOptions(quote.input.workload.kind, prompt),
    });
    row.requestId = receipt.id;
    trace = await client.wait(receipt.id, options);
  } catch (error) {
    row.status = row.requestId ? "observe_error" : "submit_error";
    if (error instanceof TokenCraftError || error instanceof TokenCraftTimeoutError) {
      row.requestId ??= error.requestId;
    }
    row.errorCode = observationError(error);
    // An interrupted submission may have queued work. Keep that slot occupied.
    return !row.requestId && error instanceof TokenCraftError
      && error.status >= 400 && error.status < 500 && error.status !== 408;
  }
  await recordTrace(client, trace, row, signal);
  return row.status === "succeeded" || row.status === "failed" || row.status === "canceled";
}

async function recordTrace(client: TokenCraft, trace: RequestTrace, row: OfferComparisonSample, signal?: AbortSignal) {
  const accepted = trace.events.find(event => event.kind === "accepted");
  const completed = trace.events.find(event => event.kind === "provider_completed" || event.kind === "mock_provider_completed");
  row.status = trace.status;
  row.errorCode = trace.upstream?.error_code ?? undefined;
  if (accepted && completed && trace.status === "succeeded") {
    const elapsed = Date.parse(completed.at) - Date.parse(accepted.at);
    if (Number.isFinite(elapsed)) row.elapsedMs = Math.max(0, elapsed);
  }
  const state = trace.cost_state;
  let cost: ObservedCost | undefined;
  if (state === "supplier_settled" && trace.supplier_settlement) {
    cost = { state, usdNanos: trace.supplier_settlement.amount_usd_nanos };
  } else if (state === "provider_reported" && trace.upstream?.reported_usd_nanos != null) {
    cost = { state, usdNanos: trace.upstream.reported_usd_nanos };
  } else if (state === "mock_reported" && trace.mock_reported_usd_nanos != null) {
    cost = { state, usdNanos: String(trace.mock_reported_usd_nanos) };
  }
  if (cost && (!row.cost || costPriority[cost.state] >= costPriority[row.cost.state])) row.cost = cost;
  const charge = trace.upstream?.provider_charge;
  if (charge && trace.provider) {
    row.providerCharge = {
      provider: trace.provider, unit: charge.unit, quantity: charge.quantity, observedAt: charge.observed_at,
    };
  }
  if (row.status !== "succeeded" || row.output) return;
  try {
    row.output = await outputShape(client, trace, signal);
    delete row.outputErrorCode;
  } catch (error) {
    row.outputErrorCode = observationError(error);
  }
}

function observationError(error: unknown): string {
  if (error instanceof TokenCraftError) return error.code;
  if (error instanceof TokenCraftTimeoutError) return "timeout";
  return "interrupted";
}

async function outputShape(client: TokenCraft, trace: RequestTrace, signal?: AbortSignal): Promise<OutputShape> {
  const kind = trace.quote.input.workload.kind;
  const mock = trace.mock_result;
  if (kind === "image") {
    if (mock?.kind === "image") return { kind, count: mock.count, aspectRatio: mock.aspect_ratio };
    const { result: { images } } = await client.imageResult(trace.id, { signal });
    return { kind, count: images.length, sizes: images.map(({ width, height }) => ({ width, height })) };
  }
  if (kind === "video") {
    if (mock?.kind === "video") {
      return { kind, durationMs: mock.duration_seconds * 1000, aspectRatio: mock.aspect_ratio, resolution: mock.resolution };
    }
    const { result: { video } } = await client.videoResult(trace.id, { signal });
    return { kind, width: video.width, height: video.height, durationMs: video.duration_ms };
  }
  if (kind === "audio") {
    if (mock?.kind === "audio") return { kind, durationMs: mock.duration_seconds * 1000, format: mock.format };
    const { result: { audio } } = await client.audioResult(trace.id, { signal });
    return { kind, durationMs: audio.duration_ms, sampleRateHz: audio.sample_rate_hz, channels: audio.channels };
  }
  if (kind === "mesh") {
    if (mock?.kind === "mesh") return { kind, format: mock.format };
    const { result: { mesh } } = await client.meshResult(trace.id, { signal });
    return { kind, meshCount: mesh.mesh_count, primitiveCount: mesh.primitive_count };
  }
  return { kind, inputTokens: trace.upstream?.input_tokens ?? null, outputTokens: trace.upstream?.output_tokens ?? null };
}

function summarize(quote: Quote, offerId: string, samples: OfferComparisonSample[]): OfferComparisonArm {
  const spend = { supplier_settled: 0n, provider_reported: 0n, mock_reported: 0n };
  const statuses: OfferComparisonArm["statuses"] = {};
  const times: number[] = [];
  let unknownCostSamples = 0;
  for (const sample of samples) {
    statuses[sample.status] = (statuses[sample.status] ?? 0) + 1;
    if (sample.cost) spend[sample.cost.state] += BigInt(sample.cost.usdNanos);
    else unknownCostSamples++;
    if (sample.elapsedMs !== undefined) times.push(sample.elapsedMs);
  }
  times.sort((a, b) => a - b);
  const median = times.length === 0 ? undefined
    : (times[Math.floor((times.length - 1) / 2)] + times[Math.floor(times.length / 2)]) / 2;
  return {
    offerId,
    quoteId: quote.id,
    catalogRevision: quote.catalog_revision,
    routing: quote.routing,
    estimatedUsdNanos: selected(quote).estimated_usd_nanos * samples.length,
    spentUsdNanos: {
      supplier_settled: String(spend.supplier_settled),
      provider_reported: String(spend.provider_reported),
      mock_reported: String(spend.mock_reported),
    },
    ...summarizeProviderCharges(samples),
    unknownCostSamples,
    statuses,
    medianCompletionMs: median,
    samples,
  };
}

function summarizeProviderCharges(samples: OfferComparisonSample[]): Pick<OfferComparisonArm, "providerCharges"> {
  const charges = new Map<string, { provider: Provider; unit: string; nanos: bigint; observations: number }>();
  for (const { providerCharge: charge } of samples) {
    if (!charge) continue;
    const decimal = /^([0-9]{1,15})(?:\.([0-9]{1,9}))?$/.exec(charge.quantity);
    if (!decimal) continue;
    const nanos = BigInt(decimal[1]) * nanosPerUnit + BigInt((decimal[2] ?? "").padEnd(9, "0"));
    const { provider, unit } = charge;
    const key = JSON.stringify([provider, unit]);
    const total = charges.get(key) ?? { provider, unit, nanos: 0n, observations: 0 };
    total.nanos += nanos;
    total.observations++;
    charges.set(key, total);
  }
  if (charges.size === 0) return {};
  return { providerCharges: [...charges.values()].map(({ provider, unit, nanos, observations }) => {
    const whole = nanos / nanosPerUnit;
    const fraction = (nanos % nanosPerUnit).toString().padStart(9, "0").replace(/0+$/, "");
    return { provider, unit, quantity: fraction ? `${whole}.${fraction}` : String(whole), observations };
  }) };
}
