import { Link } from "react-router-dom";
import { ArrowUpRight, TrendingDown, TrendingUp } from "lucide-react";
import { Badge, Card } from "@/components/ui";
import { Sparkline } from "@/components/Sparkline";
import { MODALITY_GLYPH, ModelCoverArt } from "@/components/ModelArt";
import { MODALITY_LABEL, priceLabel, type Model } from "@/lib/catalog";
import type { ModelStats } from "@/lib/model-stats";

/** Optional ranking/usage overlay for the catalog index (design §3.3). */
export function ModelCard({
  model,
  rank,
  stats,
  metric,
}: {
  model: Model;
  rank?: number;
  stats?: ModelStats;
  metric?: { label: string; value: string };
}) {
  return (
    <Link to={`/models/${model.slug}`} className="group block">
      <Card className="h-full overflow-hidden transition group-hover:border-brand/50 group-hover:shadow-glow">
        <div className="relative grid h-32 place-items-center overflow-hidden brand-gradient text-4xl">
          <span className="opacity-90">{MODALITY_GLYPH[model.modality]}</span>
          {/* Cover art layers over the gradient/glyph when the model has a sample set. */}
          <ModelCoverArt slug={model.slug} className="transition duration-300 group-hover:scale-[1.03]" />
          {rank !== undefined && (
            <div className="absolute left-3 top-3 grid h-7 min-w-7 place-items-center rounded-lg bg-black/45 px-1.5 text-sm font-bold text-white backdrop-blur-sm">
              #{rank}
            </div>
          )}
          <div className="absolute right-3 top-3 flex gap-1.5">
            {model.status === "new" && <Badge tone="green">New</Badge>}
            {model.status === "preview" && <Badge tone="amber">Preview</Badge>}
          </div>
          <ArrowUpRight className="absolute bottom-3 right-3 h-4 w-4 text-white/80 opacity-0 transition group-hover:opacity-100" />
        </div>
        <div className="space-y-2 p-4">
          <div className="flex items-center justify-between gap-2">
            <h3 className="truncate font-display font-semibold">{model.name}</h3>
            <Badge>{MODALITY_LABEL[model.modality]}</Badge>
          </div>
          <p className="text-xs uppercase tracking-wide text-muted">{model.provider}</p>
          <p className="line-clamp-2 text-sm text-muted">{model.tagline}</p>

          {stats && metric ? (
            <div className="flex items-end justify-between gap-2 pt-1">
              <div>
                <div className="text-xs text-muted">{metric.label}</div>
                <div className="flex items-center gap-1.5 text-sm font-semibold text-fg">
                  {metric.value}
                  <WoW ratio={stats.wow} />
                </div>
              </div>
              <Sparkline data={stats.sparkline} />
            </div>
          ) : (
            <p className="pt-1 text-sm font-medium text-fg">{priceLabel(model.pricing[0])}</p>
          )}
        </div>
      </Card>
    </Link>
  );
}

function WoW({ ratio }: { ratio: number }) {
  const up = ratio >= 0;
  return (
    <span className={up ? "inline-flex items-center text-xs font-medium text-emerald-500" : "inline-flex items-center text-xs font-medium text-rose-500"}>
      {up ? <TrendingUp className="h-3 w-3" /> : <TrendingDown className="h-3 w-3" />}
    </span>
  );
}
