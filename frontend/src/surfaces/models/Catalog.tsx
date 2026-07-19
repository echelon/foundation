import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { Search } from "lucide-react";
import { Container, Input } from "@/components/ui";
import { ModelCard } from "@/components/ModelCard";
import { MODELS, MODALITY_LABEL, PROVIDERS, type Modality } from "@/lib/catalog";
import { cn } from "@/lib/cn";

const MODALITIES: (Modality | "all")[] = ["all", "image", "video", "3d", "world", "chat"];

export default function Catalog() {
  const [params, setParams] = useSearchParams();
  const activeModality = (params.get("modality") as Modality | null) ?? "all";
  const [query, setQuery] = useState("");
  const [provider, setProvider] = useState<string>("all");

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    return MODELS.filter((m) => {
      if (activeModality !== "all" && m.modality !== activeModality) return false;
      if (provider !== "all" && m.provider !== provider) return false;
      if (q && !(`${m.name} ${m.provider} ${m.tags.join(" ")} ${m.tagline}`.toLowerCase().includes(q))) return false;
      return true;
    });
  }, [activeModality, provider, query]);

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
        <p className="mt-2 text-muted">{MODELS.length} models across image, video, 3D, world, and language.</p>
      </div>

      {/* ── Controls ── */}
      <div className="mb-6 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex flex-wrap gap-1.5">
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
        <div className="flex gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted" />
            <Input placeholder="Search models…" value={query} onChange={(e) => setQuery(e.target.value)} className="w-56 pl-9" />
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
      </div>

      {/* ── Grid ── */}
      {results.length ? (
        <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {results.map((m) => (
            <ModelCard key={m.slug} model={m} />
          ))}
        </div>
      ) : (
        <div className="rounded-2xl border border-dashed border-border py-20 text-center text-muted">
          No models match your filters.
        </div>
      )}
    </Container>
  );
}
