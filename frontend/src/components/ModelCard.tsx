import { Link } from "react-router-dom";
import { ArrowUpRight } from "lucide-react";
import { Badge, Card } from "@/components/ui";
import { MODALITY_LABEL, priceLabel, type Model } from "@/lib/catalog";

const MODALITY_GLYPH: Record<Model["modality"], string> = {
  image: "🖼",
  video: "🎬",
  "3d": "🧊",
  world: "🌐",
  chat: "💬",
};

export function ModelCard({ model }: { model: Model }) {
  return (
    <Link to={`/models/${model.slug}`} className="group block">
      <Card className="h-full overflow-hidden transition group-hover:border-brand/50 group-hover:shadow-glow">
        <div className="relative grid h-32 place-items-center brand-gradient text-4xl">
          <span className="opacity-90">{MODALITY_GLYPH[model.modality]}</span>
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
          <p className="pt-1 text-sm font-medium text-fg">{priceLabel(model.pricing[0])}</p>
        </div>
      </Card>
    </Link>
  );
}
