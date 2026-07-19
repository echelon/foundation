import { useState } from "react";
import { Link } from "react-router-dom";
import { ArrowRight, BarChart3 } from "lucide-react";
import { Badge, Button, Card, Container } from "@/components/ui";
import { cn } from "@/lib/cn";
import { LeaderboardTable, MoversStrip, ShareDonut } from "./components";
import { MEDIA, MEDIA_ORDER, PERIODS, compact, leaderboard, marketShare, movers, type MediaKey, type Period } from "@/lib/rankings";

export default function RankingsOverview() {
  const [period, setPeriod] = useState<Period>("month");

  const image = MEDIA.image;
  const video = MEDIA.video;

  return (
    <Container className="py-12">
      {/* ── Header ── */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <Badge tone="brand" className="mb-3">
            <BarChart3 className="h-3 w-3" /> Rankings
          </Badge>
          <h1 className="font-display text-4xl font-extrabold tracking-tight sm:text-5xl">
            The generative media <span className="brand-text">index</span>
          </h1>
          <p className="mt-3 max-w-2xl text-muted">
            Real usage across every model on Foundation — ranked by market share, broken down by task and by
            content category, day by day. Image and video, first-class alongside language.
          </p>
        </div>
        <PeriodSwitch period={period} onChange={setPeriod} />
      </div>

      {/* ── Media quick nav ── */}
      <div className="mt-8 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
        {MEDIA_ORDER.map((k) => {
          const m = MEDIA[k];
          const top = leaderboard(k, { period }).slice(0, 1)[0];
          return (
            <Link key={k} to={`/rankings/${k}`}>
              <Card className={cn("h-full p-4 transition hover:border-brand/50 hover:shadow-glow", m.featured && "border-brand/40")}>
                <div className="flex items-center justify-between">
                  <span className="text-2xl">{m.glyph}</span>
                  {m.featured && <Badge tone="brand">Featured</Badge>}
                </div>
                <div className="mt-2 font-display font-semibold">{m.label}</div>
                <div className="mt-0.5 truncate text-xs text-muted">#1 {top.model.name}</div>
                <div className="mt-2 text-xs text-muted">{compact(m.monthlyVolume)} {m.unitShort}/mo</div>
              </Card>
            </Link>
          );
        })}
      </div>

      {/* ── Image + Video leaderboards, front and center ── */}
      <div className="mt-10 grid gap-6 xl:grid-cols-2">
        <FeaturedBoard media="image" period={period} />
        <FeaturedBoard media="video" period={period} />
      </div>

      {/* ── Market share + movers ── */}
      <div className="mt-10 grid gap-6 lg:grid-cols-2">
        <Card className="p-6">
          <div className="mb-5 flex items-center justify-between">
            <div>
              <h2 className="font-display text-xl font-bold tracking-tight">Image market share</h2>
              <p className="text-sm text-muted">By lab · {image.unit}</p>
            </div>
            <Link to="/rankings/image">
              <Button variant="ghost" size="sm">
                Details <ArrowRight className="h-4 w-4" />
              </Button>
            </Link>
          </div>
          <ShareDonut slices={marketShare("image", "owner", period)} />
        </Card>
        <Card className="p-6">
          <div className="mb-5 flex items-center justify-between">
            <div>
              <h2 className="font-display text-xl font-bold tracking-tight">Video market share</h2>
              <p className="text-sm text-muted">By lab · {video.unit}</p>
            </div>
            <Link to="/rankings/video">
              <Button variant="ghost" size="sm">
                Details <ArrowRight className="h-4 w-4" />
              </Button>
            </Link>
          </div>
          <ShareDonut slices={marketShare("video", "owner", period)} />
        </Card>
      </div>

      {/* ── Movers ── */}
      <div className="mt-10">
        <h2 className="mb-4 font-display text-2xl font-bold tracking-tight">Movers of the week</h2>
        <div className="grid gap-6 lg:grid-cols-2">
          <div>
            <h3 className="mb-2 text-sm font-medium text-muted">Image</h3>
            <MoversStrip {...movers("image")} />
          </div>
          <div>
            <h3 className="mb-2 text-sm font-medium text-muted">Video</h3>
            <MoversStrip {...movers("video")} />
          </div>
        </div>
      </div>

      {/* ── LLM teaser (OpenRouter parity) ── */}
      <div className="mt-10">
        <FeaturedBoard media="llm" period={period} />
      </div>
    </Container>
  );
}

function FeaturedBoard({ media, period }: { media: MediaKey; period: Period }) {
  const m = MEDIA[media];
  const rows = leaderboard(media, { period }).slice(0, 8);
  return (
    <Card className="overflow-hidden">
      <div className="flex items-center justify-between border-b border-border px-5 py-4">
        <div className="flex items-center gap-2">
          <span className="text-xl">{m.glyph}</span>
          <div>
            <h2 className="font-display font-semibold">{m.label} models</h2>
            <p className="text-xs text-muted">Top 8 by market share</p>
          </div>
        </div>
        <Link to={`/rankings/${media}`}>
          <Button variant="outline" size="sm">
            Full board <ArrowRight className="h-4 w-4" />
          </Button>
        </Link>
      </div>
      <div className="px-2">
        <LeaderboardTable rows={rows} unitShort={m.unitShort} />
      </div>
    </Card>
  );
}

export function PeriodSwitch({ period, onChange }: { period: Period; onChange: (p: Period) => void }) {
  return (
    <div className="inline-flex rounded-lg border border-border bg-surface p-1">
      {PERIODS.map((p) => (
        <button
          key={p.key}
          onClick={() => onChange(p.key)}
          className={cn(
            "rounded-md px-3 py-1.5 text-sm font-medium transition",
            period === p.key ? "brand-gradient text-white shadow-glow" : "text-muted hover:text-fg",
          )}
        >
          {p.label}
        </button>
      ))}
    </div>
  );
}
