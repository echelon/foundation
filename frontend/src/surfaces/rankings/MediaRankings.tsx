import { useMemo, useState } from "react";
import { Link, Navigate, useParams } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import { Badge, Card, Container } from "@/components/ui";
import { cn } from "@/lib/cn";
import { PeriodSwitch } from "./Overview";
import { Chips, LeaderboardTable, MetricSwitch, MoversStrip, ShareArea, ShareDonut, StackedBars } from "./components";
import {
  MEDIA,
  MEDIA_ORDER,
  NATURAL_LANGUAGES,
  PROGRAMMING_LANGUAGES,
  compact,
  leaderboard,
  marketShare,
  movers,
  shareTimeseries,
  weeklyStack,
  type MediaKey,
  type Metric,
  type Period,
} from "@/lib/rankings";

export default function MediaRankings() {
  const { media } = useParams();
  const key = media as MediaKey;
  const [period, setPeriod] = useState<Period>("month");

  if (!media || !MEDIA_ORDER.includes(key)) return <Navigate to="/rankings" replace />;
  const m = MEDIA[key];

  const SUBTABS = useMemo(() => {
    const t = ["Leaderboard", "By task", "By category"];
    if (m.hasLanguages) t.push("By language");
    t.push("Market share", "Trends");
    return t;
  }, [m.hasLanguages]);
  const [tab, setTab] = useState<string>("Leaderboard");
  const [metricKey, setMetricKey] = useState<string>(m.metrics[0].key);
  const metric: Metric = m.metrics.find((x) => x.key === metricKey) ?? m.metrics[0];

  return (
    <Container className="py-10">
      <Link to="/rankings" className="mb-6 inline-flex items-center gap-1.5 text-sm text-muted hover:text-fg">
        <ArrowLeft className="h-4 w-4" /> All rankings
      </Link>

      {/* ── Vertical switch ── */}
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
            {compact(m.monthlyVolume)} {m.unit} this month across {leaderboard(key).length} ranked models.
          </p>
        </div>
        <PeriodSwitch period={period} onChange={setPeriod} />
      </div>

      {/* ── Hero stacked bars ── */}
      <Card className="mt-6 p-6">
        <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="font-display text-lg font-bold">Top models over time</h2>
            <p className="text-sm text-muted">Weekly {metric.label.toLowerCase()} · last 16 weeks</p>
          </div>
          <MetricSwitch metrics={m.metrics} active={metricKey} onChange={setMetricKey} />
        </div>
        <StackedBars stack={weeklyStack(key)} metric={metric} />
      </Card>

      {/* ── Sub-tabs ── */}
      <div className="scrollbar-thin mt-8 overflow-x-auto border-b border-border">
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
          <div className="space-y-4">
            <div className="flex justify-end">
              <MetricSwitch metrics={m.metrics} active={metricKey} onChange={setMetricKey} />
            </div>
            <Card className="px-2">
              <LeaderboardTable rows={leaderboard(key, { period })} metric={metric} />
            </Card>
          </div>
        )}

        {tab === "By task" && <ByDimension media={key} period={period} metric={metric} kind="task" />}
        {tab === "By category" && <ByDimension media={key} period={period} metric={metric} kind="category" />}
        {tab === "By language" && <ByLanguage media={key} period={period} metric={metric} />}

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

/** By-task or by-category board: chips + stacked bars + filtered leaderboard. */
function ByDimension({ media, period, metric, kind }: { media: MediaKey; period: Period; metric: Metric; kind: "task" | "category" }) {
  const m = MEDIA[media];
  const dims = kind === "task" ? m.tasks : m.categories;
  const [active, setActive] = useState(dims[0]?.key ?? "");
  const filter = kind === "task" ? { task: active } : { category: active };
  const rows = leaderboard(media, { period, ...filter });

  const explainer =
    kind === "task"
      ? "Every task the catalog declares gets its own board — editing, infilling, style, and more rank differently than raw generation."
      : "Semantic content categories, classified from a sample of outputs. “Best model for action scenes” is a rankable question here.";

  return (
    <div className="space-y-5">
      <Card className="border-brand/30 bg-brand/[0.05] p-4">
        <p className="text-sm text-muted">{explainer}</p>
      </Card>
      <Chips items={dims} active={active} onChange={setActive} />
      {kind === "category" && (
        <div className="flex items-center gap-2 text-xs text-muted">
          <Badge tone="amber">sampled</Badge>
          Category boards rank a classified sample of outputs, not every job.
        </div>
      )}
      <Card className="p-6">
        <h3 className="mb-4 font-display text-sm font-semibold text-muted">Weekly {metric.label.toLowerCase()} · {dims.find((d) => d.key === active)?.label}</h3>
        <StackedBars stack={weeklyStack(media, filter)} metric={metric} height={220} />
      </Card>
      <Card className="px-2">
        <LeaderboardTable rows={rows} metric={metric} />
      </Card>
    </div>
  );
}

/** By-language board (LLM: programming + natural; speech/transcription: natural). */
function ByLanguage({ media, period, metric }: { media: MediaKey; period: Period; metric: Metric }) {
  const isLLM = media === "llm";
  const [langType, setLangType] = useState<"programming" | "natural">(isLLM ? "programming" : "natural");
  const pool = langType === "programming" ? PROGRAMMING_LANGUAGES : NATURAL_LANGUAGES;
  const items = pool.map((l) => ({ key: l, label: l }));
  const [active, setActive] = useState(pool[0]);
  const rows = leaderboard(media, { period, language: active });

  return (
    <div className="space-y-5">
      <Card className="border-brand/30 bg-brand/[0.05] p-4">
        <p className="text-sm text-muted">
          Which models developers actually reach for, per language — classified from request content. Pick{" "}
          {isLLM ? "a programming or natural language" : "a language"} to re-rank the board.
        </p>
      </Card>

      {isLLM && (
        <div className="inline-flex rounded-lg border border-border bg-surface p-1">
          {(["programming", "natural"] as const).map((t) => (
            <button
              key={t}
              onClick={() => {
                setLangType(t);
                setActive((t === "programming" ? PROGRAMMING_LANGUAGES : NATURAL_LANGUAGES)[0]);
              }}
              className={cn("rounded-md px-3 py-1.5 text-sm font-medium capitalize transition", langType === t ? "brand-gradient text-white shadow-glow" : "text-muted hover:text-fg")}
            >
              {t}
            </button>
          ))}
        </div>
      )}

      <Chips items={items} active={active} onChange={setActive} />
      <div className="flex items-center gap-2 text-xs text-muted">
        <Badge tone="amber">classified</Badge>
        Language boards rank a classified sample of requests.
      </div>
      <Card className="p-6">
        <h3 className="mb-4 font-display text-sm font-semibold text-muted">Top models for {active}</h3>
        <StackedBars stack={weeklyStack(media, { language: active })} metric={metric} height={220} />
      </Card>
      <Card className="px-2">
        <LeaderboardTable rows={rows} metric={metric} />
      </Card>
    </div>
  );
}
