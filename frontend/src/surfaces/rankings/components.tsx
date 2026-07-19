import { Link } from "react-router-dom";
import { TrendingDown, TrendingUp } from "lucide-react";
import { cn } from "@/lib/cn";
import { Badge } from "@/components/ui";
import { compact, ownerColor, pct, type RankRow, type ShareSlice } from "@/lib/rankings";

/* ── Sparkline ── */
export function Sparkline({ data, className, stroke = "hsl(var(--brand))" }: { data: number[]; className?: string; stroke?: string }) {
  const w = 96;
  const h = 28;
  const min = Math.min(...data);
  const max = Math.max(...data);
  const span = max - min || 1;
  const pts = data
    .map((v, i) => `${(i / (data.length - 1)) * w},${h - ((v - min) / span) * (h - 4) - 2}`)
    .join(" ");
  const up = data[data.length - 1] >= data[0];
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} className={className} preserveAspectRatio="none">
      <polyline points={pts} fill="none" stroke={up ? stroke : "hsl(350 80% 60%)"} strokeWidth="1.75" strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}

/* ── Mover badge ── */
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

/* ── Owner avatar chip ── */
export function OwnerDot({ ownerSlug }: { ownerSlug: string }) {
  return <span className="inline-block h-2.5 w-2.5 shrink-0 rounded-full" style={{ background: ownerColor(ownerSlug) }} />;
}

/* ── Leaderboard table ── */
export function LeaderboardTable({ rows, unitShort, showSpark = true }: { rows: RankRow[]; unitShort: string; showSpark?: boolean }) {
  const maxShare = rows[0]?.share ?? 1;
  return (
    <div className="scrollbar-thin overflow-x-auto">
      <table className="w-full text-sm">
        <thead className="border-b border-border text-left text-muted">
          <tr>
            <th className="px-3 py-3 font-medium">#</th>
            <th className="px-3 py-3 font-medium">Model</th>
            <th className="px-3 py-3 font-medium">Market share</th>
            <th className="hidden px-3 py-3 text-right font-medium sm:table-cell">Output ({unitShort})</th>
            <th className="hidden px-3 py-3 text-right font-medium md:table-cell">Success</th>
            {showSpark && <th className="hidden px-3 py-3 font-medium lg:table-cell">14-day</th>}
            <th className="px-3 py-3 text-right font-medium">Trend</th>
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
                    <div className="h-full rounded-full brand-gradient" style={{ width: `${(r.share / maxShare) * 100}%` }} />
                  </div>
                  <span className="tabular-nums font-medium text-fg">{pct(r.share)}</span>
                </div>
              </td>
              <td className="hidden px-3 py-3 text-right tabular-nums text-muted sm:table-cell">{compact(r.output)}</td>
              <td className="hidden px-3 py-3 text-right tabular-nums text-muted md:table-cell">{(r.successRate * 100).toFixed(1)}%</td>
              {showSpark && (
                <td className="hidden px-3 py-3 lg:table-cell">
                  <Sparkline data={r.sparkline} />
                </td>
              )}
              <td className="px-3 py-3 text-right">
                <Mover pct={r.moverPct} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/* ── Donut chart ── */
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
            <circle
              key={s.key}
              cx={r}
              cy={r}
              r={rad}
              fill="none"
              stroke={s.key === "other" ? "hsl(var(--muted))" : ownerColor(s.ownerSlug ?? s.key)}
              strokeWidth={stroke}
              strokeDasharray={`${dash} ${circ - dash}`}
              strokeDashoffset={-offset}
            />
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

/* ── Stacked share-over-time area ── */
export function ShareArea({
  days,
  series,
  models,
}: {
  days: string[];
  series: Record<string, number | string>[];
  models: { slug: string; name: string; ownerSlug: string }[];
}) {
  const w = 720;
  const h = 260;
  const n = days.length;
  const x = (i: number) => (i / (n - 1)) * w;

  // build cumulative stacked bands (bottom→top)
  const bands = models.map((m, mi) => {
    const topPts: string[] = [];
    const botPts: string[] = [];
    for (let i = 0; i < n; i++) {
      let below = 0;
      for (let k = 0; k < mi; k++) below += Number(series[i][models[k].slug] ?? 0);
      const val = Number(series[i][m.slug] ?? 0);
      const yTop = h - ((below + val) / 100) * h;
      const yBot = h - (below / 100) * h;
      topPts.push(`${x(i)},${yTop}`);
      botPts.push(`${x(i)},${yBot}`);
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
            <Mover pct={r.moverPct} />
          </li>
        ))}
      </ul>
    </div>
  );
}
