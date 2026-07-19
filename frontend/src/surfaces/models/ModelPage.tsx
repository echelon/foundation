import { useState } from "react";
import { Link, useParams, Navigate } from "react-router-dom";
import { ArrowLeft, ImageOff } from "lucide-react";
import { Badge, Button, Card, Container } from "@/components/ui";
import { ModelCard } from "@/components/ModelCard";
import { Playground } from "./Playground";
import { MODEL_BY_SLUG, MODALITY_LABEL, priceLabel, type Model } from "@/lib/catalog";
import { endpointsForModel } from "@/lib/mock-api";
import { cn } from "@/lib/cn";

const TABS = ["Overview", "Playground", "API", "Examples", "Pricing", "Changelog"] as const;
type Tab = (typeof TABS)[number];

export default function ModelPage() {
  const { slug } = useParams();
  const model = slug ? MODEL_BY_SLUG[slug] : undefined;
  const [tab, setTab] = useState<Tab>("Playground");

  if (!model) return <Navigate to="/models" replace />;

  const related = model.related.map((s) => MODEL_BY_SLUG[s]).filter(Boolean);
  const endpoints = endpointsForModel(model);

  return (
    <Container className="py-10">
      <Link to="/models" className="mb-6 inline-flex items-center gap-1.5 text-sm text-muted hover:text-fg">
        <ArrowLeft className="h-4 w-4" /> All models
      </Link>

      {/* ── Header ── */}
      <div className="flex flex-col gap-6 border-b border-border pb-8 md:flex-row md:items-start md:justify-between">
        <div className="max-w-2xl">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="font-display text-4xl font-extrabold tracking-tight">{model.name}</h1>
            {model.status === "new" && <Badge tone="green">New</Badge>}
            {model.status === "preview" && <Badge tone="amber">Preview</Badge>}
          </div>
          <p className="mt-1 text-sm uppercase tracking-wide text-muted">
            {model.provider} · {MODALITY_LABEL[model.modality]}
          </p>
          <p className="mt-4 text-lg text-muted">{model.tagline}</p>
          <div className="mt-4 flex flex-wrap gap-1.5">
            {model.tags.map((t) => (
              <Badge key={t}>{t}</Badge>
            ))}
          </div>
        </div>
        <Card className="w-full shrink-0 p-5 md:w-64">
          <div className="text-xs text-muted">Starting at</div>
          <div className="mt-1 font-display text-2xl font-bold">{priceLabel(model.pricing[0])}</div>
          <div className="mt-4 space-y-1.5 text-sm text-muted">
            <div className="flex justify-between">
              <span>Released</span>
              <span className="text-fg">{model.released}</span>
            </div>
            {model.contextTokens && (
              <div className="flex justify-between">
                <span>Context</span>
                <span className="text-fg">{(model.contextTokens / 1000).toLocaleString()}K</span>
              </div>
            )}
            <div className="flex justify-between">
              <span>Modality</span>
              <span className="text-fg">{MODALITY_LABEL[model.modality]}</span>
            </div>
          </div>
          <Button className="mt-5 w-full" onClick={() => setTab("Playground")}>
            Try in playground
          </Button>
        </Card>
      </div>

      {/* ── Tabs ── */}
      <div className="scrollbar-thin sticky top-16 z-30 -mx-5 mt-6 overflow-x-auto border-b border-border bg-bg/80 px-5 backdrop-blur sm:-mx-8 sm:px-8">
        <div className="flex gap-1">
          {TABS.map((t) => (
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

      {/* ── Tab content ── */}
      <div className="py-8">
        {tab === "Overview" && (
          <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_18rem]">
            <div className="prose-invert max-w-none">
              <p className="text-lg leading-relaxed text-fg">{model.description}</p>
              <h3 className="mt-8 font-display text-xl font-semibold">Good for</h3>
              <div className="mt-3 flex flex-wrap gap-1.5">
                {model.tags.map((t) => (
                  <Badge key={t} tone="brand">
                    {t}
                  </Badge>
                ))}
              </div>
            </div>
            <Card className="h-fit p-5">
              <h3 className="font-display font-semibold">At a glance</h3>
              <dl className="mt-3 space-y-2 text-sm">
                <Row k="Provider" v={model.provider} />
                <Row k="Modality" v={MODALITY_LABEL[model.modality]} />
                <Row k="Status" v={model.status.toUpperCase()} />
                <Row k="Released" v={model.released} />
                <Row k="Slug" v={model.slug} mono />
              </dl>
            </Card>
          </div>
        )}

        {tab === "Playground" && <Playground model={model} />}

        {tab === "API" && (
          <div className="space-y-6">
            <p className="max-w-2xl text-muted">
              Call {model.name} directly by its slug, route to it via a modality endpoint, or price a request
              first with the <code className="rounded bg-surface2 px-1.5 py-0.5 text-sm">/cost</code> twin.
            </p>
            <Card className="overflow-hidden">
              <div className="scrollbar-thin overflow-x-auto">
                <table className="w-full text-sm">
                  <tbody>
                    {endpoints.map((e) => (
                      <tr key={e.path} className="border-b border-border/60 last:border-0">
                        <td className="px-4 py-3 align-top">
                          <span className="rounded bg-brand/12 px-2 py-0.5 font-mono text-xs font-semibold text-brand">{e.method}</span>
                        </td>
                        <td className="whitespace-nowrap px-4 py-3 align-top font-mono text-[13px] text-fg">{e.path}</td>
                        <td className="px-4 py-3 align-top text-muted">{e.blurb}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
            <Card className="overflow-hidden">
              <div className="border-b border-border px-4 py-2.5 text-xs text-muted">Example request</div>
              <pre className="scrollbar-thin overflow-x-auto p-5 text-[13px] leading-relaxed">
                <code>{exampleRequest(model)}</code>
              </pre>
            </Card>
          </div>
        )}

        {tab === "Examples" && (
          <div className="rounded-2xl border border-dashed border-border py-24 text-center">
            <ImageOff className="mx-auto mb-4 h-10 w-10 text-muted/60" />
            <h3 className="font-display text-lg font-semibold">No example outputs yet</h3>
            <p className="mx-auto mt-2 max-w-sm text-sm text-muted">
              Curated example generations will appear here. Until then, try the model yourself in the playground.
            </p>
            <Button variant="outline" className="mt-5" onClick={() => setTab("Playground")}>
              Open playground
            </Button>
          </div>
        )}

        {tab === "Pricing" && (
          <Card className="overflow-hidden">
            <table className="w-full text-sm">
              <thead className="border-b border-border bg-surface2/50 text-left text-muted">
                <tr>
                  <th className="px-4 py-3 font-medium">Price</th>
                  <th className="px-4 py-3 font-medium">Unit</th>
                  <th className="px-4 py-3 font-medium">Notes</th>
                </tr>
              </thead>
              <tbody>
                {model.pricing.map((p, i) => (
                  <tr key={i} className="border-b border-border/60 last:border-0">
                    <td className="px-4 py-3 font-medium text-fg">{priceLabel(p)}</td>
                    <td className="px-4 py-3 font-mono text-[13px] text-muted">{p.unit}</td>
                    <td className="px-4 py-3 text-muted">{p.note ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Card>
        )}

        {tab === "Changelog" && (
          <div className="space-y-4">
            {model.changelog.map((c) => (
              <Card key={c.version} className="flex gap-4 p-5">
                <Badge tone="brand" className="h-fit">
                  v{c.version}
                </Badge>
                <div>
                  <div className="text-sm text-muted">{c.date}</div>
                  <p className="mt-1 text-fg">{c.note}</p>
                </div>
              </Card>
            ))}
          </div>
        )}
      </div>

      {/* ── Related ── */}
      {related.length > 0 && (
        <div className="border-t border-border pt-10">
          <h2 className="mb-5 font-display text-2xl font-bold tracking-tight">Related models</h2>
          <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {related.map((m) => (
              <ModelCard key={m.slug} model={m} />
            ))}
          </div>
        </div>
      )}
    </Container>
  );
}

function Row({ k, v, mono }: { k: string; v: string; mono?: boolean }) {
  return (
    <div className="flex justify-between gap-4">
      <dt className="text-muted">{k}</dt>
      <dd className={cn("text-right text-fg", mono && "font-mono text-[13px]")}>{v}</dd>
    </div>
  );
}

function exampleRequest(model: Model): string {
  if (model.modality === "chat") {
    return `curl https://api.foundation.ai/v1/chat/completions \\
  -H "Authorization: Bearer $FOUNDATION_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "model": "${model.slug}",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 1024
  }'`;
  }
  return `curl https://api.foundation.ai/v1/models/${model.slug}/generate \\
  -H "Authorization: Bearer $FOUNDATION_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "prompt": "a beautiful landscape at golden hour",
    "end_user": "cust_a2f_user_5512"
  }'`;
}
