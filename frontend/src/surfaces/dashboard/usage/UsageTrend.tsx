import { useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { Link } from "react-router-dom";
import { ArrowUpRight, BarChart3, ChevronLeft, ChevronRight } from "lucide-react";
import { Button, Card, Select } from "@/components/ui";
import { cn } from "@/lib/cn";
import type { UsageReport } from "@/surfaces/comparison/api";
import { availableMetrics, metrics, shortDay, usageBuckets, windowLabel, type MetricId, type UsageWindow } from "./usageData";

export type Interval = "auto" | "day" | "week";

export function UsageTrend({ report, groups, metric, interval, onMetric, onInterval, requestLink }: {
  report: UsageReport; groups: UsageReport["groups"]; metric: MetricId; interval: Interval;
  onMetric: (metric: MetricId) => void; onInterval: (interval: Interval) => void;
  requestLink?: (window: UsageWindow) => string;
}) {
  const choices = availableMetrics(report);
  const selected = choices.includes(metric) ? metric : "requests";
  const days = (Date.parse(report.window.to) - Date.parse(report.window.from)) / 86_400_000;
  const resolution = interval === "auto" ? days > 31 ? "week" : "day" : interval;
  const definition = metrics[selected];
  const total = definition.read(report.totals);

  return (
    <Card className="min-w-0 overflow-hidden">
      <div className="flex flex-wrap items-start justify-between gap-4 px-4 pb-4 pt-5 sm:px-5">
        <div>
          <h2 className="flex items-center gap-2 font-display text-lg font-semibold"><BarChart3 className="h-4 w-4 text-brand" aria-hidden /> Usage over time</h2>
          <p className="mt-2 flex flex-wrap items-baseline gap-x-2 gap-y-1">
            <span className="break-all font-display text-3xl font-bold tabular-nums tracking-tight">{total === null ? "—" : definition.format(total)}</span>
            <span className="text-sm text-muted">{definition.label}</span>
          </p>
        </div>
        <div className="grid max-w-full grid-cols-[minmax(0,1fr)_auto] gap-2">
          <Select aria-label="Chart metric" className="h-9 truncate rounded-lg" value={selected} onChange={(event) => onMetric(event.target.value as MetricId)}>
            {choices.map((key) => <option key={key} value={key}>{metrics[key].label}</option>)}
          </Select>
          <Select aria-label="Chart interval" className="h-9 rounded-lg" value={interval} onChange={(event) => onInterval(event.target.value as Interval)}>
            <option value="auto">Auto</option><option value="day">Daily</option><option value="week">Weekly</option>
          </Select>
        </div>
      </div>
      {report.totals.requests === 0 ? (
        <div className="mx-4 mb-4 grid min-h-52 place-content-center gap-1 rounded-xl border border-dashed border-border px-4 text-center sm:mx-5">
          <p className="font-medium">No matching requests in this period.</p>
          <p className="text-sm text-muted">Choose a wider date range or clear a filter.</p>
        </div>
      ) : <BarPlot key={`${report.window.from}/${report.window.to}/${resolution}`} window={report.window} groups={groups}
        metric={selected} interval={resolution} requestLink={requestLink} />}
    </Card>
  );
}

function BarPlot({ window, groups, metric, interval, requestLink }: {
  window: UsageWindow; groups: UsageReport["groups"]; metric: MetricId; interval: "day" | "week";
  requestLink?: (window: UsageWindow) => string;
}) {
  const buckets = usageBuckets(window, groups, metric, interval);
  const [picked, setPicked] = useState(() => buckets.reduce((last, bucket, index) => bucket.requests > 0 ? index : last, 0));
  const [hovered, setHovered] = useState<number | null>(null);
  const plot = useRef<HTMLDivElement>(null);
  const definition = metrics[metric];
  const maximum = buckets.reduce((max, bucket) => Math.max(max, Number(bucket.value ?? 0n)), 0);
  const step = tickStep(maximum);
  const top = step * 4;
  const active = hovered ?? picked;
  const bucket = buckets[active];
  const axisFormat = top < 1_000 ? definition.format : (value: bigint) => {
    if (metric === "requests" || metric === "outputs") return compactNumber.format(value);
    return top >= 1_000_000_000_000 ? `$${compactNumber.format(Number(value) / 1_000_000_000)}` : definition.format(value);
  };
  const labelEvery = Math.ceil(buckets.length / 7);

  useLayoutEffect(() => {
    const scroller = plot.current;
    const button = scroller?.querySelectorAll("button")[picked];
    if (!scroller || !button) return;
    const view = scroller.getBoundingClientRect();
    const bar = button.getBoundingClientRect();
    if (bar.left < view.left) scroller.scrollLeft -= view.left - bar.left;
    if (bar.right > view.right) scroller.scrollLeft += bar.right - view.right;
  }, [picked]);

  function selectBar(index: number, moveFocus = false) {
    const next = Math.min(buckets.length - 1, Math.max(0, index));
    setHovered(null);
    setPicked(next);
    if (moveFocus) plot.current?.querySelectorAll("button")[next]?.focus({ preventScroll: true });
  }

  function navigate(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const direction = { ArrowLeft: index - 1, ArrowRight: index + 1, Home: 0, End: buckets.length - 1 };
    if (!Object.hasOwn(direction, event.key)) return;
    event.preventDefault();
    selectBar(direction[event.key as keyof typeof direction], true);
  }

  if (!bucket) return null;
  return (
    <div className="px-4 sm:px-5">
      <p className="mb-2 text-xs font-medium text-muted">{interval === "day" ? "Daily" : "Weekly"} · UTC</p>
      <div className="flex gap-2 sm:gap-3">
        <div className="relative h-44 shrink-0 text-right font-mono text-[10px] tabular-nums text-muted sm:h-52" aria-hidden>
          {[4, 3, 2, 1, 0].map((tick) => <div key={tick} className="relative h-1/4">
            {(maximum > 0 || tick === 0) && <span className="relative -top-2.5">{axisFormat(BigInt(step * tick))}</span>}
          </div>)}
        </div>
        <div className="min-w-0 flex-1 overflow-x-auto pb-2" ref={plot}>
          <div className="relative" style={{ minWidth: Math.max(0, buckets.length * (buckets.length > 14 ? 14 : 0)) }}>
            <div className="pointer-events-none absolute inset-x-0 top-0 h-44 border-b border-border sm:h-52" aria-hidden>
              {[0, 1, 2, 3].map((line) => <div key={line} className="h-1/4 border-t border-border/80" />)}
            </div>
            <div role="group" aria-label={`${definition.label} by ${interval === "day" ? "UTC day" : "UTC week"}`}
              className="relative grid gap-x-0.5 sm:gap-x-1" style={{ gridTemplateColumns: `repeat(${buckets.length}, minmax(0, 1fr))` }}
              onPointerLeave={() => setHovered(null)}>
              {buckets.map((item, index) => (
                <div key={item.from} className="min-w-0">
                  <button type="button" tabIndex={index === picked ? 0 : -1} aria-pressed={index === picked}
                    aria-label={`${windowLabel(item)}, ${definition.label}: ${item.value === null ? "no amount recorded" : definition.format(item.value)}`}
                    onPointerEnter={(event) => { if (event.pointerType === "mouse") setHovered(index); }}
                    onFocus={() => { setHovered(null); setPicked(index); }}
                    onClick={() => { setHovered(null); setPicked(index); }} onKeyDown={(event) => navigate(event, index)}
                    className={cn("relative flex h-44 w-full items-end justify-center rounded-t-md px-[1px] outline-none motion-safe:transition-colors focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-brand sm:h-52",
                      index === active && "bg-brand/10")}>
                    <span aria-hidden className={cn("w-full max-w-16 rounded-t-md", item.value === null || item.value === 0n ? "bg-border" :
                      index === active ? "bg-brand" : "bg-brand/60")}
                      style={{ height: `${(Number(item.value ?? 0n) / top) * 100}%`, minHeight: item.value && item.value > 0n ? 3 : 1 }} />
                  </button>
                  {index % labelEvery === 0 && <div aria-hidden className={cn("mt-2 overflow-visible whitespace-nowrap text-[10px] leading-4 text-muted sm:block sm:text-left sm:text-xs",
                    index % (2 * labelEvery) !== 0 && "hidden", index === buckets.length - 1 && "text-right")}>
                    {shortDay(item.from)}
                  </div>}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
      <div className="mt-2 flex min-h-16 flex-wrap items-center gap-x-4 gap-y-2 border-t border-border py-3">
        <div className="flex shrink-0 items-center gap-1" aria-label="Inspect chart periods">
          <Button type="button" variant="outline" size="sm" className="w-8 rounded-md px-0" aria-label="Previous chart period"
            disabled={picked <= 0} onClick={() => selectBar(picked - 1)}><ChevronLeft className="h-4 w-4" aria-hidden /></Button>
          <Button type="button" variant="outline" size="sm" className="w-8 rounded-md px-0" aria-label="Next chart period"
            disabled={picked >= buckets.length - 1} onClick={() => selectBar(picked + 1)}><ChevronRight className="h-4 w-4" aria-hidden /></Button>
        </div>
        <div className="min-w-28 flex-1 text-sm">
          <div className="text-xs text-muted">{windowLabel(bucket)} · UTC</div>
          <div className="break-words font-semibold tabular-nums">
            {bucket.value === null ? "No amount recorded" : `${definition.format(bucket.value)} ${definition.label.toLowerCase()}`}
          </div>
        </div>
        {requestLink && bucket.requests > 0 && <Link to={requestLink(bucket)}
          className="inline-flex items-center gap-1 text-xs font-semibold text-brand hover:underline focus-visible:outline-brand sm:text-sm">
          Inspect requests <ArrowUpRight className="h-3.5 w-3.5" aria-hidden />
        </Link>}
      </div>
    </div>
  );
}

const compactNumber = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 });

function tickStep(maximum: number) {
  if (maximum <= 4) return 1;
  const target = maximum / 4;
  const magnitude = 10 ** Math.floor(Math.log10(target));
  return magnitude * ([1, 2, 5, 10].find((step) => magnitude * step >= target) ?? 10);
}
