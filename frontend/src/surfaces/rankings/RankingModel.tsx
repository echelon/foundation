import { Link, Navigate, useParams } from "react-router-dom";
import { ArrowLeft, ExternalLink } from "lucide-react";
import { Badge, Button, Card, Container } from "@/components/ui";
import { Sparkline, Mover, OwnerDot } from "./components";
import { MEDIA, compact, modelDetail, pct } from "@/lib/rankings";
import { MODEL_BY_SLUG } from "@/lib/catalog";
import { cn } from "@/lib/cn";

export default function RankingModel() {
  const { slug } = useParams();
  const detail = slug ? modelDetail(slug) : undefined;
  if (!detail) return <Navigate to="/rankings" replace />;

  const { media, model, row, topTasks, topCategories, topApps, topLanguages } = detail;
  const m = MEDIA[media];
  const inCatalog = !!MODEL_BY_SLUG[model.slug];

  return (
    <Container className="py-10">
      <Link to={`/rankings/${media}`} className="mb-6 inline-flex items-center gap-1.5 text-sm text-muted hover:text-fg">
        <ArrowLeft className="h-4 w-4" /> {m.label} rankings
      </Link>

      {/* ── Header ── */}
      <div className="flex flex-col gap-6 border-b border-border pb-8 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <div className="flex items-center gap-2 text-sm text-muted">
            <OwnerDot ownerSlug={model.ownerSlug} /> {model.owner} · {m.label}
          </div>
          <h1 className="mt-1 font-display text-4xl font-extrabold tracking-tight">{model.name}</h1>
          <div className="mt-3 flex flex-wrap items-center gap-3">
            <Badge tone="brand">Rank #{row.rank}</Badge>
            <span className="text-sm text-muted">
              {pct(row.share)} market share · <Mover pct={row.moverPct} /> this week
            </span>
          </div>
        </div>
        {inCatalog && (
          <Link to={`/models/${model.slug}`}>
            <Button variant="outline">
              Open model page <ExternalLink className="h-4 w-4" />
            </Button>
          </Link>
        )}
      </div>

      {/* ── Stat grid ── */}
      <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat label={`Output (${m.unitShort})`} value={compact(row.output)} sub="this month" />
        <Stat label="Requests" value={compact(row.jobs)} sub="this month" />
        <Stat label="Success rate" value={`${(row.successRate * 100).toFixed(1)}%`} sub="terminal jobs" />
        <Stat label="Latency p50 / p95" value={`${fmtMs(row.p50)} / ${fmtMs(row.p95)}`} sub="queue → done" />
      </div>

      {/* ── Trend ── */}
      <Card className="mt-6 p-6">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="font-display font-semibold">14-day usage trend</h2>
            <p className="text-sm text-muted">Relative output volume · first seen {row.firstSeen}</p>
          </div>
          <Mover pct={row.moverPct} />
        </div>
        <div className="mt-4">
          <Sparkline data={row.sparkline} className="h-24 w-full" />
        </div>
      </Card>

      {/* ── Breakdowns ── */}
      <div className={cn("mt-6 grid gap-6", topLanguages ? "lg:grid-cols-2 xl:grid-cols-4" : "lg:grid-cols-3")}>
        <BreakdownCard title="Top tasks" rows={topTasks.map((t) => ({ label: t.label, share: t.share }))} />
        <BreakdownCard title="Top content categories" rows={topCategories.map((c) => ({ label: c.label, share: c.share }))} note="classified sample" />
        {topLanguages && (
          <BreakdownCard title={media === "llm" ? "Top languages" : "Top spoken languages"} rows={topLanguages} note="classified" />
        )}
        <BreakdownCard title="Top apps" rows={topApps.map((a) => ({ label: a.app, share: a.share }))} note="opt-in" />
      </div>
    </Container>
  );
}

function Stat({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <Card className="p-5">
      <div className="text-sm text-muted">{label}</div>
      <div className="mt-1.5 font-display text-2xl font-bold">{value}</div>
      <div className="mt-1 text-xs text-muted">{sub}</div>
    </Card>
  );
}

function BreakdownCard({ title, rows, note }: { title: string; rows: { label: string; share: number }[]; note?: string }) {
  const max = Math.max(...rows.map((r) => r.share)) || 1;
  return (
    <Card className="p-5">
      <div className="mb-4 flex items-center justify-between">
        <h3 className="font-display font-semibold">{title}</h3>
        {note && <Badge tone={note === "opt-in" ? "muted" : "amber"}>{note}</Badge>}
      </div>
      <ul className="space-y-3">
        {rows.map((r) => (
          <li key={r.label}>
            <div className="mb-1 flex items-center justify-between text-sm">
              <span className="text-fg">{r.label}</span>
              <span className="tabular-nums text-muted">{pct(r.share)}</span>
            </div>
            <div className="h-1.5 overflow-hidden rounded-full bg-surface2">
              <div className="h-full rounded-full brand-gradient" style={{ width: `${(r.share / max) * 100}%` }} />
            </div>
          </li>
        ))}
      </ul>
    </Card>
  );
}

function fmtMs(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
}
