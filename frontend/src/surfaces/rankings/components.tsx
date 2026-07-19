import { useState } from "react";
import { Link } from "react-router-dom";
import { TrendingDown, TrendingUp } from "lucide-react";
import { cn } from "@/lib/cn";
import { Badge } from "@/components/ui";
import {
  compact,
  metricValue,
  ownerColor,
  pct,
  signedPct,
  type Metric,
  type RankRow,
  type ShareSlice,
  type WeeklyStack,
} from "@/lib/rankings";

/* ── Sparkline ── */
export function Sparkline({ data, className, stroke = "hsl(var(--brand))" }: { data: number[]; className?: string; stroke?: string }) {
  const w = 96;
  const h = 28;
  const min = Math.min(...data);
  const max = Math.max(...data);
  const span = max - min || 1;
  const pts = data.map((v, i) => `${(i / (data.length - 1)) * w},${h - ((v - min) / span) * (h - 4) - 2}`).join(" ");
  const up = data[data.length - 1] >= data[0];
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} className={className} preserveAspectRatio="none">
      <polyline points={pts} fill="none" stroke={up ? stroke : "hsl(350 80% 62%)"} strokeWidth="1.75" strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}

/* ── WoW / mover badge ── */
export function WoW({ ratio }: { ratio: number }) {
  const up = ratio >= 0;
  return (
    <span className={cn("inline-flex items-center gap-0.5 tabular-nums text-xs font-semibold", up ? "text-emerald-500" : "text-rose-500")}>
      {up ? <TrendingUp className="h-3.5 w-3.5" /> : <TrendingDown className="h-3.5 w-3.5" />}
      {signedPct(ratio, 0)}
    </span>
  );
}
export function Mover({ pct: p }: { pct: number }) {
  const up = p >= 0;
  return (
    <span className={cn("inline-flex items-center gap-1 text-xs font-semibold", up ? "text-emerald-500" : "text-rose-500")}>
      {up ? <TrendingUp className="h-3.5 w-3.5" /> : <TrendingDown className="h-3.5 w-3.5" />}
      {up ? "+" : ""}
      {p.toFixed(1)}
    </span>
  );
}

export function OwnerDot({ ownerSlug }: { ownerSlug: string }) {
  return <span className="inline-block h-2.5 w-2.5 shrink-0 rounded-full" style={{ background: ownerColor(ownerSlug) }} />;
}

/* ── Leaderboard table (metric-aware, WoW column) ── */
export function LeaderboardTable({ rows, metric }: { rows: RankRow[]; metric: Metric }) {
  const maxShare = rows[0]?.share ?? 1;
  return (
    <div className="scrollbar-thin overflow-x-auto">
      <table className="w-full text-sm">
        <thead className="border-b border-border text-left text-muted">
          <tr>
            <th className="px-3 py-3 font-medium">#</th>
            <th className="px-3 py-3 font-medium">Model</th>
            <th className="px-3 py-3 font-medium">Market share</th>
            <th className="px-3 py-3 text-right font-medium">{metric.label} ({metric.unitShort})</th>
            <th className="hidden px-3 py-3 text-right font-medium sm:table-cell">Weekly</th>
            <th className="hidden px-3 py-3 font-medium lg:table-cell">14-day</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.model.slug} className="border-b border-border/60 last:border-0 hover:bg-surface2/40">
              <td className="px-3 py-3 font-mono text-muted">{r.rank}</td>
              <td className="px-3 py-3">
                <Link to={`/rankings/models/${r.model.slug}`} className="flex items-center gap-2 font-medium text-fg hover:text-brand">
                  <OwnerDot ownerSlug={r.model.ownerSlug} />
                  <span className="truncate">{r.model.name}</span>
                </Link>
                <div className="pl-[18px] text-xs text-muted">{r.model.owner}</div>
              </td>
              <td className="px-3 py-3">
                <div className="flex items-center gap-2">
                  <div className="h-2 w-24 overflow-hidden rounded-full bg-surface2">
                    <div className="h-full rounded-full" style={{ width: `${(r.share / maxShare) * 100}%`, background: ownerColor(r.model.ownerSlug) }} />
                  </div>
                  <span className="tabular-nums font-medium text-fg">{pct(r.share)}</span>
                </div>
              </td>
              <td className="px-3 py-3 text-right tabular-nums font-medium text-fg">{compact(metricValue(r, metric))}</td>
              <td className="hidden px-3 py-3 text-right sm:table-cell">
                <WoW ratio={r.wowPct} />
              </td>
              <td className="hidden px-3 py-3 lg:table-cell">
                <Sparkline data={r.sparkline} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/* ── Signature stacked BAR chart (weekly, hover breakdown) ── */
export function StackedBars({ stack, metric, height = 300 }: { stack: WeeklyStack; metric: Metric; height?: number }) {
  const [hover, setHover] = useState<number | null>(null);
  const maxTotal = Math.max(...stack.totals) || 1;
  const n = stack.weeks.length;
  const div = metric.divisor ?? 1;

  const active = hover ?? n - 1;
  const breakdown = stack.segments
    .map((s) => ({ name: s.name, ownerSlug: s.ownerSlug, value: s.values[active] }))
    .filter((x) => x.value > 0)
    .sort((a, b) => b.value - a.value);
  const activeTotal = stack.totals[active];

  return (
    <div>
      <div className="relative flex items-end gap-[3px]" style={{ height }} onMouseLeave={() => setHover(null)}>
        {stack.weeks.map((wk, wi) => {
          const total = stack.totals[wi];
          const colH = (total / maxTotal) * height;
          return (
            <div
              key={wk}
              className="group relative flex flex-1 cursor-pointer flex-col-reverse"
              style={{ height: colH }}
              onMouseEnter={() => setHover(wi)}
            >
              {stack.segments.map((seg) => {
                const v = seg.values[wi];
                if (v <= 0) return null;
                return (
                  <div
                    key={seg.slug}
                    style={{ height: `${(v / total) * 100}%`, background: ownerColor(seg.ownerSlug) }}
                    className={cn("w-full transition-opacity", hover !== null && hover !== wi ? "opacity-40" : "opacity-100")}
                  />
                );
              })}
            </div>
          );
        })}

        {/* tooltip */}
        {hover !== null && (
          <div
            className="pointer-events-none absolute bottom-full z-10 mb-2 w-56 -translate-x-1/2 rounded-xl border border-border bg-surface p-3 text-xs shadow-glow"
            style={{ left: `${Math.min(88, Math.max(12, ((hover + 0.5) / n) * 100))}%` }}
          >
            <div className="mb-2 flex items-center justify-between">
              <span className="font-semibold text-fg">Week of {stack.weeks[hover]}</span>
              <span className="text-muted">{compact(activeTotal / div)} {metric.unitShort}</span>
            </div>
            <ul className="space-y-1">
              {breakdown.slice(0, 6).map((b) => (
                <li key={b.name} className="flex items-center gap-1.5">
                  <span className="inline-block h-2 w-2 rounded-sm" style={{ background: ownerColor(b.ownerSlug) }} />
                  <span className="flex-1 truncate text-fg">{b.name}</span>
                  <span className="tabular-nums text-muted">{pct(b.value / activeTotal, 0)}</span>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>

      {/* x-axis */}
      <div className="mt-2 flex gap-[3px] text-[10px] text-muted">
        {stack.weeks.map((wk, wi) => (
          <span key={wk} className="flex-1 text-center">
            {wi % 3 === 0 ? wk : ""}
          </span>
        ))}
      </div>

      {/* legend */}
      <div className="mt-4 flex flex-wrap gap-x-4 gap-y-1.5">
        {stack.segments.map((s) => (
          <span key={s.slug} className="inline-flex items-center gap-1.5 text-xs text-muted">
            <span className="inline-block h-2.5 w-2.5 rounded-sm" style={{ background: ownerColor(s.ownerSlug) }} />
            {s.name}
          </span>
        ))}
      </div>
    </div>
  );
}

/* ── Donut ── */
export function ShareDonut({ slices, size = 220 }: { slices: ShareSlice[]; size?: number }) {
  const top = slices.slice(0, 8);
  const rest = slices.slice(8);
  const restShare = rest.reduce((s, x) => s + x.share, 0);
  const all = [...top, ...(restShare > 0 ? [{ key: "other", label: "Other", share: restShare, output: 0, ownerSlug: "other" }] : [])];
  const total = all.reduce((s, x) => s + x.share, 0) || 1;
  const r = size / 2;
  const stroke = size * 0.16;
  const rad = r - stroke / 2;
  const circ = 2 * Math.PI * rad;
  let offset = 0;
  return (
    <div className="flex flex-col items-center gap-5 sm:flex-row sm:items-center">
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="shrink-0 -rotate-90">
        {all.map((s) => {
          const frac = s.share / total;
          const dash = frac * circ;
          const el = (
            <circle key={s.key} cx={r} cy={r} r={rad} fill="none" stroke={s.key === "other" ? "hsl(var(--muted))" : ownerColor(s.ownerSlug ?? s.key)} strokeWidth={stroke} strokeDasharray={`${dash} ${circ - dash}`} strokeDashoffset={-offset} />
          );
          offset += dash;
          return el;
        })}
      </svg>
      <ul className="w-full space-y-1.5 text-sm">
        {all.map((s) => (
          <li key={s.key} className="flex items-center gap-2">
            <span className="inline-block h-2.5 w-2.5 rounded-sm" style={{ background: s.key === "other" ? "hsl(var(--muted))" : ownerColor(s.ownerSlug ?? s.key) }} />
            <span className="flex-1 truncate text-fg">{s.label}</span>
            <span className="tabular-nums font-medium text-muted">{pct(s.share)}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/* ── Stacked share-over-time area (day-by-day) ── */
export function ShareArea({ days, series, models }: { days: string[]; series: Record<string, number | string>[]; models: { slug: string; name: string; ownerSlug: string }[] }) {
  const w = 720;
  const h = 260;
  const n = days.length;
  const x = (i: number) => (i / (n - 1)) * w;
  const bands = models.map((m, mi) => {
    const topPts: string[] = [];
    const botPts: string[] = [];
    for (let i = 0; i < n; i++) {
      let below = 0;
      for (let k = 0; k < mi; k++) below += Number(series[i][models[k].slug] ?? 0);
      const val = Number(series[i][m.slug] ?? 0);
      topPts.push(`${x(i)},${h - ((below + val) / 100) * h}`);
      botPts.push(`${x(i)},${h - (below / 100) * h}`);
    }
    const d = `M ${topPts.join(" L ")} L ${botPts.reverse().join(" L ")} Z`;
    return { d, color: ownerColor(m.ownerSlug), name: m.name, slug: m.slug };
  });
  return (
    <div>
      <div className="scrollbar-thin overflow-x-auto">
        <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} className="w-full min-w-[560px]" preserveAspectRatio="none">
          {bands.map((b) => (
            <path key={b.slug} d={b.d} fill={b.color} fillOpacity={0.85} stroke="hsl(var(--bg))" strokeWidth={0.5} />
          ))}
        </svg>
      </div>
      <div className="mt-2 flex justify-between text-[10px] text-muted">
        {days.filter((_, i) => i % 2 === 0).map((d) => (
          <span key={d}>{d.split(" ")[1]}</span>
        ))}
      </div>
      <div className="mt-3 flex flex-wrap gap-x-4 gap-y-1.5">
        {models.map((m) => (
          <span key={m.slug} className="inline-flex items-center gap-1.5 text-xs text-muted">
            <span className="inline-block h-2.5 w-2.5 rounded-sm" style={{ background: ownerColor(m.ownerSlug) }} />
            {m.name}
          </span>
        ))}
      </div>
    </div>
  );
}

/* ── Movers strip ── */
export function MoversStrip({ up, down }: { up: RankRow[]; down: RankRow[] }) {
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <MoverList title="Gainers" rows={up} />
      <MoverList title="Decliners" rows={down} />
    </div>
  );
}
function MoverList({ title, rows }: { title: string; rows: RankRow[] }) {
  return (
    <div className="rounded-xl border border-border bg-surface p-4">
      <div className="mb-3 flex items-center justify-between">
        <h4 className="font-display text-sm font-semibold">{title}</h4>
        <Badge>this week</Badge>
      </div>
      <ul className="space-y-2">
        {rows.map((r) => (
          <li key={r.model.slug} className="flex items-center justify-between gap-2">
            <Link to={`/rankings/models/${r.model.slug}`} className="flex items-center gap-2 truncate text-sm text-fg hover:text-brand">
              <OwnerDot ownerSlug={r.model.ownerSlug} />
              <span className="truncate">{r.model.name}</span>
            </Link>
            <WoW ratio={r.wowPct} />
          </li>
        ))}
      </ul>
    </div>
  );
}

/* ── Chip row (task / category / language selectors) ── */
export function Chips<T extends string>({ items, active, onChange }: { items: { key: T; label: string }[]; active: T; onChange: (k: T) => void }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {items.map((d) => (
        <button
          key={d.key}
          onClick={() => onChange(d.key)}
          className={cn(
            "rounded-lg px-3 py-1.5 text-sm font-medium transition",
            active === d.key ? "brand-gradient text-white shadow-glow" : "bg-surface2 text-muted hover:text-fg",
          )}
        >
          {d.label}
        </button>
      ))}
    </div>
  );
}

/* ── Metric switch ── */
export function MetricSwitch({ metrics, active, onChange }: { metrics: Metric[]; active: string; onChange: (k: string) => void }) {
  if (metrics.length < 2) return null;
  return (
    <div className="inline-flex rounded-lg border border-border bg-surface p-1">
      {metrics.map((m) => (
        <button
          key={m.key}
          onClick={() => onChange(m.key)}
          className={cn("rounded-md px-3 py-1.5 text-xs font-medium transition", active === m.key ? "bg-surface2 text-fg" : "text-muted hover:text-fg")}
        >
          {m.label}
        </button>
      ))}
    </div>
  );
}
