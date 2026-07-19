import { useState } from "react";
import { Link, Navigate, useParams } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import { Badge, Card, Container } from "@/components/ui";
import { cn } from "@/lib/cn";
import { PeriodSwitch } from "./Overview";
import { LeaderboardTable, MoversStrip, ShareArea, ShareDonut } from "./components";
import {
  MEDIA,
  MEDIA_ORDER,
  compact,
  leaderboard,
  marketShare,
  movers,
  shareTimeseries,
  type MediaKey,
  type Period,
} from "@/lib/rankings";

const SUBTABS = ["Leaderboard", "By task", "By category", "Market share", "Trends"] as const;
type SubTab = (typeof SUBTABS)[number];

export default function MediaRankings() {
  const { media } = useParams();
  const key = media as MediaKey;
  const [tab, setTab] = useState<SubTab>("Leaderboard");
  const [period, setPeriod] = useState<Period>("month");

  if (!media || !MEDIA_ORDER.includes(key)) return <Navigate to="/rankings" replace />;
  const m = MEDIA[key];

  return (
    <Container className="py-10">
      <Link to="/rankings" className="mb-6 inline-flex items-center gap-1.5 text-sm text-muted hover:text-fg">
        <ArrowLeft className="h-4 w-4" /> All rankings
      </Link>

      {/* ── Media switch + header ── */}
      <div className="mb-6 flex flex-wrap gap-1.5">
        {MEDIA_ORDER.map((k) => (
          <Link
            key={k}
            to={`/rankings/${k}`}
            className={cn(
              "rounded-lg px-3 py-1.5 text-sm font-medium transition",
              k === key ? "brand-gradient text-white shadow-glow" : "bg-surface2 text-muted hover:text-fg",
            )}
          >
            {MEDIA[k].glyph} {MEDIA[k].label}
          </Link>
        ))}
      </div>

      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="font-display text-4xl font-extrabold tracking-tight">{m.label} rankings</h1>
          <p className="mt-2 text-muted">
            {compact(m.monthlyVolume)} {m.unit} this month across {MEDIA_ORDER.includes(key) ? leaderboard(key).length : 0} ranked
            models.
          </p>
        </div>
        <PeriodSwitch period={period} onChange={setPeriod} />
      </div>

      {/* ── Sub-tabs ── */}
      <div className="scrollbar-thin mt-6 overflow-x-auto border-b border-border">
        <div className="flex gap-1">
          {SUBTABS.map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={cn(
                "relative whitespace-nowrap px-3 py-3 text-sm font-medium transition",
                tab === t ? "text-fg" : "text-muted hover:text-fg",
              )}
            >
              {t}
              {tab === t && <span className="absolute inset-x-2 bottom-0 h-0.5 rounded-full brand-gradient" />}
            </button>
          ))}
        </div>
      </div>

      <div className="py-8">
        {tab === "Leaderboard" && (
          <Card className="px-2">
            <LeaderboardTable rows={leaderboard(key, { period })} unitShort={m.unitShort} />
          </Card>
        )}

        {tab === "By task" && <ByDimension media={key} period={period} kind="task" />}
        {tab === "By category" && <ByDimension media={key} period={period} kind="category" />}

        {tab === "Market share" && (
          <div className="grid gap-6 lg:grid-cols-2">
            <Card className="p-6">
              <h2 className="mb-5 font-display text-lg font-bold">By lab</h2>
              <ShareDonut slices={marketShare(key, "owner", period)} />
            </Card>
            <Card className="p-6">
              <h2 className="mb-5 font-display text-lg font-bold">By model</h2>
              <ShareDonut slices={marketShare(key, "model", period)} />
            </Card>
          </div>
        )}

        {tab === "Trends" && (
          <div className="space-y-6">
            <Card className="p-6">
              <div className="mb-4">
                <h2 className="font-display text-lg font-bold">Share over time</h2>
                <p className="text-sm text-muted">Day-by-day market share of the top models · last 14 days</p>
              </div>
              {(() => {
                const ts = shareTimeseries(key, 6);
                return <ShareArea days={ts.days} series={ts.series} models={ts.models} />;
              })()}
            </Card>
            <div>
              <h2 className="mb-4 font-display text-lg font-bold">Movers of the week</h2>
              <MoversStrip {...movers(key)} />
            </div>
          </div>
        )}
      </div>
    </Container>
  );
}

/** By-task or by-category: a chip selector + the filtered leaderboard. */
function ByDimension({ media, period, kind }: { media: MediaKey; period: Period; kind: "task" | "category" }) {
  const m = MEDIA[media];
  const dims = kind === "task" ? m.tasks : m.categories;
  const [active, setActive] = useState(dims[0]?.key ?? "");

  const rows = leaderboard(media, {
    period,
    ...(kind === "task" ? { task: active } : { category: active }),
  });

  const explainer =
    kind === "task"
      ? "Every task the catalog declares gets its own board — editing, infilling, style, and more rank differently than raw generation."
      : "Semantic content categories, classified from a sample of outputs. “Best model for action scenes” is a rankable question here.";

  return (
    <div className="space-y-5">
      <Card className="flex items-start gap-3 border-brand/30 bg-brand/[0.05] p-4">
        <p className="text-sm text-muted">{explainer}</p>
      </Card>

      <div className="flex flex-wrap gap-1.5">
        {dims.map((d) => (
          <button
            key={d.key}
            onClick={() => setActive(d.key)}
            className={cn(
              "rounded-lg px-3 py-1.5 text-sm font-medium transition",
              active === d.key ? "brand-gradient text-white shadow-glow" : "bg-surface2 text-muted hover:text-fg",
            )}
          >
            {d.label}
          </button>
        ))}
      </div>

      {kind === "category" && (
        <div className="flex items-center gap-2 text-xs text-muted">
          <Badge tone="amber">sampled</Badge>
          Category boards rank a classified sample of outputs, not every job.
        </div>
      )}

      <Card className="px-2">
        <LeaderboardTable rows={rows} unitShort={m.unitShort} />
      </Card>
    </div>
  );
}
