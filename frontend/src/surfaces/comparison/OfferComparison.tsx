import { Check, CircleDashed, GitCompareArrows } from "lucide-react";
import { Badge, Card } from "@/components/ui";
import { policySources } from "@/lib/routing-policy";
import { formatNanoUsd, providerLabel, usd, type ModelQuote, type Quote, type Reason } from "./api";
import { PolicySummary, toleranceLabel } from "./PolicySummary";
import { useDeadlineClock } from "./QuoteExpiry";

const reasons: Record<Reason, string> = {
  eligible: "Eligible",
  provider_excluded: "Provider excluded",
  provider_policy_excluded: "Blocked by policy",
  provider_region_unmet: "Region requirement unmet",
  provider_retention_unmet: "Retention requirement unmet",
  workload_unsupported: "Workload unsupported",
  aspect_ratio_unsupported: "Ratio unsupported",
  above_cost_cap: "Over budget",
  price_stale: "Price expired",
  queue_cooling_down: "Queue cooling down",
  provider_credential_unavailable: "Provider key unavailable",
  insufficient_model_ratings: "Fewer than 10 ratings",
  below_model_quality: "Below the quality threshold",
  below_model_usage: "Below the usage threshold",
};

const retryTime = new Intl.DateTimeFormat(undefined, {
  hour: "numeric", minute: "2-digit", second: "2-digit", timeZoneName: "short",
});
const rateTime = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit", second: "2-digit" });

export function OfferComparison({ quote, loading, publicEstimate = false }: {
  quote: Quote | ModelQuote | null;
  loading: boolean;
  publicEstimate?: boolean;
}) {
  const performance = quote?.performance_window;
  const preferred = Boolean(quote?.routing?.provider_preference?.value.length);
  const bySettledCost = quote?.routing?.route_ranking?.value === "cost_per_completion";
  const metric = bySettledCost ? "settled cost per completion" : "completion rate and time";
  const windowLabel = bySettledCost ? "completed UTC days" : "days";
  const ranking = performance?.applied
    ? `Ranked by ${metric} over the last ${performance.window_days} ${windowLabel}, within provider priority.`
    : preferred ? "Eligible preferred providers come first. Estimated cost breaks ties."
    : "Ranked by estimated total for this request.";
  const workload = quote?.input.workload;
  const requestedRatio = workload?.kind === "image" || workload?.kind === "video" ? workload.aspect_ratio : null;
  const measuredSelection = performance?.applied
    ? (bySettledCost ? "Lowest settled cost" : "Reliability selection")
    : null;
  const rows = !quote ? [] : "prompt_mode" in quote
    ? quote.candidates.map((row, index) => ({
      ...row, key: String(index), offer_tolerance: undefined, rateDeadline: row.rate_valid_until,
      name: row.selected ? measuredSelection ?? (row.preference_rank ? "Preferred route" : "Lowest eligible") : "Alternate route",
    }))
    : quote.candidates.map((row) => ({ ...row, key: row.offer_id, rateDeadline: row.rate_evidence?.valid_until,
      name: row.model ? `${row.model} · ${providerLabel(row.provider)}` : providerLabel(row.provider), selected: row.offer_id === quote.selected_offer_id }));
  const now = useDeadlineClock([quote?.expires_at, ...rows.map(row => row.rateDeadline)]);
  const expired = Date.parse(quote?.expires_at ?? "") <= now
    || rows.some(row => row.reason === "eligible" && Date.parse(row.rateDeadline ?? "") <= now);
  const selected = rows.find((row) => row.selected);
  const footer = selected
    ? publicEstimate ? "Project policies and provider credentials apply when you submit a request."
    : selected.live
      ? "Estimated price in USD. This live route can incur provider charges."
      : selected.self_hosted
        ? "Estimated compute cost in USD. This route runs on the operator’s inference server."
        : "Fixture prices in USD. Demo requests incur zero charges."
    : rows.some((row) => row.available_after)
      ? "Refresh the quote after a queue's retry time."
      : rows.some((row) => row.reason === "provider_credential_unavailable")
        ? "An organization admin can configure the provider key in Projects. Refresh the quote afterward."
        : "No offer satisfies these inputs. Adjust your constraints to compare again.";

  return (
    <Card className="min-w-0 overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-5 sm:px-7">
        <div>
          <h2 className="flex items-center gap-2 text-lg font-semibold"><GitCompareArrows className="h-4 w-4 text-brand" aria-hidden /> Route estimates</h2>
          <p className="mt-1 text-sm text-muted">{ranking}</p>
        </div>
        {expired ? <Badge tone="amber">Quote expired</Badge>
          : selected && <Badge tone="brand"><Check className="h-3 w-3" aria-hidden /> Route selected</Badge>}
      </div>
      <div aria-live="polite" aria-busy={loading}>
        {quote ? (
          <>
            {quote.routing && <div className="border-b border-border px-5 py-4 sm:px-7"><PolicySummary routing={quote.routing} /></div>}
            <table className="w-full text-left text-sm">
              <thead className="bg-surface2/60 text-xs text-muted">
                <tr><th scope="col" className="px-5 py-3 sm:px-7">Route</th><th scope="col" className="px-2 py-3">Estimated</th><th scope="col" className="px-3 py-3 sm:px-7">Eligibility</th></tr>
              </thead>
              <tbody className="divide-y divide-border">
                {rows.map((candidate) => {
                  const isSelected = candidate.selected && !expired;
                  const deadline = Date.parse(candidate.rateDeadline ?? "");
                  const hasDeadline = Number.isFinite(deadline);
                  const eligibility = hasDeadline && candidate.reason === "price_stale" ? null : reasons[candidate.reason];
                  const cost = candidate.performance?.cost;
                  const adapted = requestedRatio && candidate.applied_aspect_ratio && candidate.applied_aspect_ratio !== requestedRatio;
                  return (
                    <tr key={candidate.key} className={isSelected ? "bg-brand/10" : undefined}>
                      <th scope="row" className="break-words px-5 py-4 text-fg sm:px-7">
                        {candidate.name}
                        {candidate.preference_rank && <span className="mt-1 block text-xs font-normal text-muted">Provider priority {candidate.preference_rank}</span>}
                        {candidate.output_size && <span className="mt-1 block text-xs font-normal text-muted">
                          {candidate.output_size.width} × {candidate.output_size.height} px
                        </span>}
                        {adapted && <span className="mt-1 block font-mono text-xs font-normal text-muted">{requestedRatio} → {candidate.applied_aspect_ratio}</span>}
                        {candidate.offer_tolerance && <span className="mt-1 block text-xs font-normal text-muted">
                          {toleranceLabel(candidate.offer_tolerance)} · {policySources[candidate.offer_tolerance.source]}
                        </span>}
                      </th>
                      <td className="px-2 py-4 font-mono tabular-nums">{usd(candidate.estimated_usd_nanos)}</td>
                      <td className="px-3 py-4 text-xs text-muted sm:px-7">
                        {isSelected ? <span className="font-medium text-brand">Selected</span>
                          : candidate.selected ? "Previous selection" : eligibility}
                        {hasDeadline && <span className="mt-1 block text-xs">
                          {deadline <= now || candidate.reason === "price_stale" ? "Price expired " : "Price valid until "}
                          <time dateTime={candidate.rateDeadline}>{rateTime.format(deadline)}</time>
                        </span>}
                        {candidate.performance && <div className="mt-1 space-y-0.5 tabular-nums">
                          <div>{candidate.performance.samples
                            ? `${candidate.performance.succeeded} of ${candidate.performance.samples} completed`
                            : "No comparable runs"}</div>
                          {candidate.performance.median_ms !== null && (
                            <div>{(candidate.performance.median_ms / 1000).toLocaleString(undefined, { maximumFractionDigits: 3 })} s median</div>
                          )}
                          {candidate.performance.unconfirmed > 0 && <div>{candidate.performance.unconfirmed} unconfirmed</div>}
                          {cost && <div className="border-t border-border pt-1">
                            <div>{candidate.performance.samples ? `${cost.settled} of ${candidate.performance.samples} direct runs settled` : "No comparable direct runs"}</div>
                            {cost.settled === candidate.performance.samples && candidate.performance.succeeded > 0 && cost.settled_usd_nanos !== null && (
                              <div className="break-all">
                                {BigInt(cost.settled_usd_nanos) % BigInt(candidate.performance.succeeded) !== 0n && "≈"}
                                {formatNanoUsd(cost.settled_usd_nanos, candidate.performance.succeeded)} per completion
                              </div>
                            )}
                          </div>}
                        </div>}
                        {candidate.available_after && (
                          <time className="mt-1 block font-mono" dateTime={candidate.available_after}>
                            {retryTime.format(new Date(candidate.available_after))}
                          </time>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            <p className="border-t border-border px-5 py-4 text-xs text-muted sm:px-7">
              {footer}
              {performance && !performance.applied && (
                <span className="mt-1 block">{bySettledCost
                  ? `Cost ranking needs matching outputs, complete settlements on at least ${performance.minimum_samples} direct runs per route, and a confirmed completion.`
                  : `Reliability ranking needs two eligible routes at the same priority and ${performance.minimum_samples} comparable runs per route.`}
                </span>
              )}
            </p>
          </>
        ) : (
          <div className="flex min-h-52 flex-col items-center justify-center gap-3 px-6 text-center text-sm text-muted">
            <CircleDashed className="h-6 w-6 text-brand" aria-hidden />
            {loading ? "Comparing offers…" : "Set your requirements to see eligible offers."}
          </div>
        )}
      </div>
    </Card>
  );
}
