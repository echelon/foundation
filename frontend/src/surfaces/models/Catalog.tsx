import { useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { LayoutGrid, Rows3, Search, TrendingDown, TrendingUp } from "lucide-react";
import { Badge, Container, Input } from "@/components/ui";
import { Sparkline } from "@/components/Sparkline";
import { ModelCard } from "@/components/ModelCard";
import { MODELS, MODALITY_LABEL, PROVIDERS, priceLabel, type Modality } from "@/lib/catalog";
import { SORTS, rankModels, sortMetric, type RankedModel, type SortKey } from "@/lib/model-stats";
import { cn } from "@/lib/cn";

const MODALITIES: (Modality | "all")[] = ["all", "image", "video", "3d", "world", "chat"];

export default function Catalog() {
  const [params, setParams] = useSearchParams();
  const activeModality = (params.get("modality") as Modality | null) ?? "all";
  const [query, setQuery] = useState("");
  const [provider, setProvider] = useState<string>("all");
  const [sort, setSort] = useState<SortKey>("generations");
  const [view, setView] = useState<"grid" | "table">("grid");

  const ranked = useMemo(() => {
    const q = query.trim().toLowerCase();
    const filtered = MODELS.filter((m) => {
      if (activeModality !== "all" && m.modality !== activeModality) return false;
      if (provider !== "all" && m.provider !== provider) return false;
      if (q && !`${m.name} ${m.provider} ${m.tags.join(" ")} ${m.tagline}`.toLowerCase().includes(q)) return false;
      return true;
    });
    return rankModels(filtered, sort);
  }, [activeModality, provider, query, sort]);

  const setModality = (m: Modality | "all") => {
    const next = new URLSearchParams(params);
    if (m === "all") next.delete("modality");
    else next.set("modality", m);
    setParams(next, { replace: true });
  };

  return (
    <Container className="py-12">
      <div className="mb-8">
        <h1 className="font-display text-4xl font-extrabold tracking-tight">Model catalog</h1>
        <p className="mt-2 text-muted">
          {MODELS.length} models across image, video, 3D, world, and language — ranked by live usage.
        </p>
      </div>

      {/* ── Modality tabs ── */}
      <div className="mb-4 flex flex-wrap gap-1.5">
        {MODALITIES.map((m) => (
          <button
            key={m}
            onClick={() => setModality(m)}
            className={cn(
              "rounded-lg px-3 py-1.5 text-sm font-medium transition",
              activeModality === m ? "brand-gradient text-white shadow-glow" : "bg-surface2 text-muted hover:text-fg",
            )}
          >
            {m === "all" ? "All" : MODALITY_LABEL[m]}
          </button>
        ))}
      </div>

      {/* ── Controls: search · provider · sort · view ── */}
      <div className="mb-6 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex flex-wrap gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted" />
            <Input placeholder="Search models…" value={query} onChange={(e) => setQuery(e.target.value)} className="w-52 pl-9" />
          </div>
          <select
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
            className="h-10 rounded-lg border border-border bg-bg px-3 text-sm text-fg outline-none focus:border-brand focus:ring-2 focus:ring-brand/25"
          >
            <option value="all">All providers</option>
            {PROVIDERS.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-2">
          <label className="text-sm text-muted">Rank by</label>
          <select
            value={sort}
            onChange={(e) => setSort(e.target.value as SortKey)}
            className="h-10 rounded-lg border border-border bg-bg px-3 text-sm font-medium text-fg outline-none focus:border-brand focus:ring-2 focus:ring-brand/25"
          >
            {SORTS.map((s) => (
              <option key={s.key} value={s.key}>
                {s.label}
              </option>
            ))}
          </select>
          <div className="inline-flex rounded-lg border border-border bg-surface p-1">
            <ViewButton active={view === "grid"} onClick={() => setView("grid")} icon={LayoutGrid} label="Gallery" />
            <ViewButton active={view === "table"} onClick={() => setView("table")} icon={Rows3} label="Table" />
          </div>
        </div>
      </div>

      {/* ── Results ── */}
      {!ranked.length ? (
        <div className="rounded-2xl border border-dashed border-border py-20 text-center text-muted">No models match your filters.</div>
      ) : view === "grid" ? (
        <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {ranked.map((rm) => (
            <ModelCard key={rm.model.slug} model={rm.model} rank={rm.rank} stats={rm.stats} metric={sortMetric(rm, sort)} />
          ))}
        </div>
      ) : (
        <TableView ranked={ranked} sort={sort} />
      )}

      <p className="mt-6 text-xs text-muted">
        Usage figures are a mock of the hot catalog data (<code className="rounded bg-surface2 px-1 py-0.5">catalog_stats.json</code>) —
        volume, price-weighted value, and ranks that the real index serves from a single cached payload.
      </p>
    </Container>
  );
}

function ViewButton({ active, onClick, icon: Icon, label }: { active: boolean; onClick: () => void; icon: typeof LayoutGrid; label: string }) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      className={cn("flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-medium transition", active ? "bg-surface2 text-fg" : "text-muted hover:text-fg")}
    >
      <Icon className="h-4 w-4" />
      <span className="hidden sm:inline">{label}</span>
    </button>
  );
}

const GLYPH: Record<Modality, string> = { image: "🖼", video: "🎬", "3d": "🧊", world: "🌐", chat: "💬" };

function TableView({ ranked, sort }: { ranked: RankedModel[]; sort: SortKey }) {
  const navigate = useNavigate();
  const metricLabel = sortMetric(ranked[0], sort).label;
  return (
    <div className="scrollbar-thin overflow-x-auto rounded-2xl border border-border">
      <table className="w-full text-sm">
        <thead className="border-b border-border bg-surface2/40 text-left text-muted">
          <tr>
            <th className="px-4 py-3 font-medium">#</th>
            <th className="px-4 py-3 font-medium">Model</th>
            <th className="hidden px-4 py-3 font-medium sm:table-cell">Modality</th>
            <th className="hidden px-4 py-3 font-medium md:table-cell">Price</th>
            <th className="px-4 py-3 text-right font-medium">{metricLabel}</th>
            <th className="hidden px-4 py-3 text-right font-medium sm:table-cell">Weekly</th>
            <th className="hidden px-4 py-3 lg:table-cell">14-day</th>
          </tr>
        </thead>
        <tbody>
          {ranked.map((rm) => {
            const m = rm.model;
            const met = sortMetric(rm, sort);
            const up = rm.stats.wow >= 0;
            return (
              <tr
                key={m.slug}
                onClick={() => navigate(`/models/${m.slug}`)}
                className="cursor-pointer border-b border-border/60 last:border-0 hover:bg-surface2/40"
              >
                <td className="px-4 py-3 font-mono text-muted">{rm.rank}</td>
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <span className="text-lg">{GLYPH[m.modality]}</span>
                    <div className="min-w-0">
                      <div className="flex items-center gap-1.5">
                        <span className="truncate font-medium text-fg">{m.name}</span>
                        {m.status === "new" && <Badge tone="green">New</Badge>}
                      </div>
                      <div className="text-xs text-muted">{m.provider}</div>
                    </div>
                  </div>
                </td>
                <td className="hidden px-4 py-3 sm:table-cell">
                  <Badge>{MODALITY_LABEL[m.modality]}</Badge>
                </td>
                <td className="hidden px-4 py-3 text-muted md:table-cell">{priceLabel(m.pricing[0])}</td>
                <td className="px-4 py-3 text-right font-semibold text-fg">{met.value}</td>
                <td className="hidden px-4 py-3 text-right sm:table-cell">
                  <span className={cn("inline-flex items-center gap-0.5 text-xs font-semibold", up ? "text-emerald-500" : "text-rose-500")}>
                    {up ? <TrendingUp className="h-3.5 w-3.5" /> : <TrendingDown className="h-3.5 w-3.5" />}
                    {up ? "+" : ""}
                    {(rm.stats.wow * 100).toFixed(0)}%
                  </span>
                </td>
                <td className="hidden px-4 py-3 lg:table-cell">
                  <Sparkline data={rm.stats.sparkline} />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
