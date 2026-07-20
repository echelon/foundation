import { useState } from "react";
import { Link, useParams, Navigate, useLocation, useNavigate } from "react-router-dom";
import { ArrowLeft, BarChart3, Bot, FileText, ImageOff, Plug } from "lucide-react";
import { Badge, Button, Card, Container } from "@/components/ui";
import { ModelCard } from "@/components/ModelCard";
import { CodeTabs } from "@/components/CodeBlock";
import { Playground } from "./Playground";
import { Gallery, type RemixValues } from "./Gallery";
import { MODEL_BY_SLUG, MODALITY_LABEL, priceLabel, type Model } from "@/lib/catalog";
import { endpointsForModel, type EndpointDoc } from "@/lib/mock-api";
import { snippets } from "@/lib/codegen";
import { examplesForModel } from "@/lib/gallery";
import { cn } from "@/lib/cn";

const TABS = ["Overview", "Playground", "API", "Examples", "Pricing", "Changelog"] as const;
type Tab = (typeof TABS)[number];

/** Tabs sync to the URL hash (linkable, back/forward history) — matches the design's #anchors. */
const TAB_HASH: Record<Tab, string> = {
  Overview: "overview",
  Playground: "playground",
  API: "api",
  Examples: "examples",
  Pricing: "pricing",
  Changelog: "changelog",
};
function tabFromHash(hash: string): Tab {
  const h = hash.replace(/^#/, "").toLowerCase();
  return (TABS.find((t) => TAB_HASH[t] === h) as Tab) ?? "Playground";
}

export default function ModelPage() {
  const { slug } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const model = slug ? MODEL_BY_SLUG[slug] : undefined;
  const tab = tabFromHash(location.hash);
  const setTab = (t: Tab) => navigate(`#${TAB_HASH[t]}`);
  const [remix, setRemix] = useState<{ values: RemixValues; nonce: number } | null>(null);

  if (!model) return <Navigate to="/models" replace />;

  const applyRemix = (values: RemixValues) => {
    setRemix((r) => ({ values, nonce: (r?.nonce ?? 0) + 1 }));
    setTab("Playground");
  };

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
          <Link
            to={`/rankings/${rankingsMediaFor(model.modality)}`}
            className="mt-4 inline-flex items-center gap-1.5 text-sm font-medium text-brand hover:underline"
          >
            <BarChart3 className="h-4 w-4" /> See {MODALITY_LABEL[model.modality]} rankings
          </Link>
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

        {tab === "Playground" && <Playground key={remix?.nonce ?? 0} model={model} initialValues={remix?.values} />}

        {tab === "API" && <ApiTab model={model} endpoints={endpoints} />}

        {tab === "Examples" &&
          (examplesForModel(model.slug).length ? (
            <Gallery slug={model.slug} onRemix={applyRemix} />
          ) : (
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
          ))}

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

/** Map a catalog modality to a rankings media key. */
function rankingsMediaFor(modality: Model["modality"]): string {
  return modality === "chat" ? "llm" : modality;
}

/* ── API tab: code samples (cURL / JS / Python) + endpoints + agents CTA ── */
function ApiTab({ model, endpoints }: { model: Model; endpoints: EndpointDoc[] }) {
  return (
    <div className="space-y-8">
      <div>
        <p className="max-w-2xl text-muted">
          Call {model.name} directly by its slug, route to it via a modality endpoint, or price a request first
          with the <code className="rounded bg-surface2 px-1.5 py-0.5 text-sm">/cost</code> twin. One key
          authenticates everything.
        </p>
        <CodeTabs className="mt-4" tabs={snippets(model)} />
      </div>

      <div>
        <h3 className="mb-3 font-display font-semibold">Endpoints</h3>
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
      </div>

      <AgentsCTA model={model} />
    </div>
  );
}

/** Call to action for LLMs, Agents, and MCP. */
function AgentsCTA({ model }: { model: Model }) {
  const items = [
    {
      icon: FileText,
      title: "llms.txt",
      body: "Machine-readable docs for this exact model — params, schema, pricing, and copy-paste snippets, built for LLMs.",
      action: `/models/${model.slug}/llms.txt`,
      cta: "View llms.txt",
    },
    {
      icon: Plug,
      title: "MCP server",
      body: "Connect Foundation to any MCP client so agents can call this model as a tool, with usage scoped to your key.",
      action: "https://docs.foundation.ai/mcp",
      cta: "Add MCP server",
    },
    {
      icon: Bot,
      title: "Coding agents",
      body: "Drop the snippets straight into Claude Code, Cursor, or your own agent — the API is OpenAI-compatible where it counts.",
      action: "https://docs.foundation.ai/agents",
      cta: "Agent guide",
    },
  ];
  return (
    <Card className="relative overflow-hidden p-6">
      <div className="pointer-events-none absolute inset-0 brand-gradient opacity-[0.06]" />
      <div className="relative">
        <Badge tone="brand" className="mb-2">
          Built for agents
        </Badge>
        <h3 className="font-display text-xl font-bold tracking-tight">Use {model.name} from LLMs, agents & MCP</h3>
        <p className="mt-1 max-w-2xl text-sm text-muted">
          Every model ships agent-ready. Point an LLM at its <code className="rounded bg-surface2 px-1 py-0.5">llms.txt</code>,
          expose it as an MCP tool, or wire it into your coding agent.
        </p>
        <div className="mt-5 grid gap-4 md:grid-cols-3">
          {items.map((it) => (
            <div key={it.title} className="rounded-xl border border-border bg-surface p-4">
              <div className="mb-3 grid h-9 w-9 place-items-center rounded-lg bg-brand/12 text-brand">
                <it.icon className="h-5 w-5" />
              </div>
              <div className="font-display font-semibold">{it.title}</div>
              <p className="mt-1 text-sm text-muted">{it.body}</p>
              <a href={it.action} className="mt-3 inline-flex text-sm font-medium text-brand hover:underline">
                {it.cta} →
              </a>
            </div>
          ))}
        </div>
      </div>
    </Card>
  );
}
