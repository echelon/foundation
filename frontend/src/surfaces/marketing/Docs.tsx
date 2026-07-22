import { Badge, Card, Container } from "@/components/ui";
import { MODALITY_LABEL } from "@/lib/catalog";

/** Endpoint reference — the public API surface, per the design-doc endpoint split. */
const ENDPOINT_GROUPS: { title: string; blurb: string; rows: { method: string; path: string; desc: string }[] }[] = [
  {
    title: "Model-specific endpoints",
    blurb: "Pin one exact model and version. Deterministic — you always get the model you named.",
    rows: [
      { method: "POST", path: "/v1/models/{model}/generate", desc: "Generate with a specific model (image, 3D, or world)." },
      { method: "POST", path: "/v1/models/{model}/generate/{modality}", desc: "Disambiguate when a model exposes more than one output modality." },
      { method: "POST", path: "/v1/models/{model}/generate/cost", desc: "Cost twin — same payload, returns the price, runs nothing." },
      { method: "GET", path: "/v1/models/{model}", desc: "Public catalog read: metadata, pricing, and supported modalities." },
    ],
  },
  {
    title: "Modality endpoints",
    blurb: "Ask for a media type and let Foundation route to the best model — or name one to pin it.",
    rows: [
      { method: "POST", path: "/v1/any/image", desc: "Generate an image with the best-fit or named model." },
      { method: "POST", path: "/v1/any/video", desc: "Generate a video clip." },
      { method: "POST", path: "/v1/any/3d", desc: "Generate a 3D mesh." },
      { method: "POST", path: "/v1/any/world", desc: "Generate an explorable 3D world (Gaussian splat)." },
      { method: "POST", path: "/v1/any/{media}/cost", desc: "Cost twin for any modality endpoint." },
    ],
  },
  {
    title: "Language (chat) endpoints",
    blurb: "OpenAI-compatible — point an existing SDK at Foundation and set model to a catalog slug.",
    rows: [
      { method: "POST", path: "/v1/chat/completions", desc: "Chat completions across every LLM in the catalog." },
      { method: "POST", path: "/v1/chat/completions/cost", desc: "Estimate the cost of a chat request before running it." },
    ],
  },
  {
    title: "Catalog & model-page endpoints",
    blurb: "Everything the model pages render is public and available over the API.",
    rows: [
      { method: "GET", path: "/v1/models", desc: "List the full catalog with pricing, tags, and status." },
      { method: "GET", path: "/v1/models/{model}", desc: "Full detail for one model." },
      { method: "GET", path: "/v1/models/{model}/changelog", desc: "Version history for a model." },
      { method: "GET", path: "/v1/models/{model}/examples", desc: "Curated example outputs for the gallery." },
    ],
  },
];

const COST_UNITS: { unit: string; applies: string; formula: string }[] = [
  { unit: "per_image", applies: "Image models", formula: "unit price × number of images × quality/resolution tier" },
  { unit: "per_video_second", applies: "Video models", formula: "unit price × clip duration in seconds" },
  { unit: "per_mesh", applies: "3D mesh models", formula: "unit price × meshes (texture resolution tier applies)" },
  { unit: "per_splat", applies: "3D world models", formula: "unit price × generated worlds" },
  { unit: "per_million_tokens_in / _out", applies: "Language models", formula: "(input_tokens × in-rate + output_tokens × out-rate) ÷ 1,000,000" },
];

export default function Docs() {
  return (
    <Container className="max-w-5xl py-16">
      <Badge tone="brand" className="mb-4">
        Reference
      </Badge>
      <h1 className="font-display text-4xl font-extrabold tracking-tight">API & cost reference</h1>
      <p className="mt-4 max-w-2xl text-muted">
        Foundation exposes three families of endpoints — model-specific, modality, and language — plus a public
        catalog. Every generate endpoint has a <code className="rounded bg-surface2 px-1.5 py-0.5 text-sm">/cost</code>{" "}
        twin, and every request can carry an <code className="rounded bg-surface2 px-1.5 py-0.5 text-sm">end_user</code>{" "}
        identifier so usage is traceable down to your own customers.
      </p>

      {/* ── Auth note ── */}
      <Card className="mt-8 p-5">
        <h2 className="font-display font-semibold">Authentication</h2>
        <p className="mt-2 text-sm text-muted">
          Send your key as a bearer token. Use <code className="rounded bg-surface2 px-1.5 py-0.5">sk_live_…</code>{" "}
          in production and <code className="rounded bg-surface2 px-1.5 py-0.5">sk_test_…</code> for sandbox traffic.
        </p>
        <pre className="scrollbar-thin mt-3 overflow-x-auto rounded-lg bg-surface2/60 p-4 text-[13px]">
          <code>Authorization: Bearer sk_live_xxxxxxxxxxxxxxxx</code>
        </pre>
      </Card>

      {/* ── Endpoints ── */}
      <div className="mt-12 space-y-10">
        {ENDPOINT_GROUPS.map((g) => (
          <section key={g.title}>
            <h2 className="font-display text-2xl font-bold tracking-tight">{g.title}</h2>
            <p className="mt-1 text-sm text-muted">{g.blurb}</p>
            <Card className="mt-4 overflow-hidden">
              <div className="scrollbar-thin overflow-x-auto">
                <table className="w-full text-sm">
                  <tbody>
                    {g.rows.map((r) => (
                      <tr key={r.path} className="border-b border-border/60 last:border-0">
                        <td className="px-4 py-3 align-top">
                          <span className="rounded bg-brand/12 px-2 py-0.5 font-mono text-xs font-semibold text-brand">
                            {r.method}
                          </span>
                        </td>
                        <td className="whitespace-nowrap px-4 py-3 align-top font-mono text-[13px] text-fg">{r.path}</td>
                        <td className="px-4 py-3 align-top text-muted">{r.desc}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          </section>
        ))}
      </div>

      {/* ── Cost structures ── */}
      <section className="mt-14">
        <h2 className="font-display text-2xl font-bold tracking-tight">Cost structures</h2>
        <p className="mt-1 text-sm text-muted">
          Every response includes a <code className="rounded bg-surface2 px-1.5 py-0.5">cost</code> object with the
          charged amount and a breakdown. Costs draw down prepaid credits; tax is applied at top-up.
        </p>
        <Card className="mt-4 overflow-hidden">
          <div className="scrollbar-thin overflow-x-auto">
            <table className="w-full text-sm">
              <thead className="border-b border-border bg-surface2/50 text-left text-muted">
                <tr>
                  <th className="px-4 py-3 font-medium">Metering unit</th>
                  <th className="px-4 py-3 font-medium">Applies to</th>
                  <th className="px-4 py-3 font-medium">How it's computed</th>
                </tr>
              </thead>
              <tbody>
                {COST_UNITS.map((c) => (
                  <tr key={c.unit} className="border-b border-border/60 last:border-0">
                    <td className="whitespace-nowrap px-4 py-3 font-mono text-[13px] text-fg">{c.unit}</td>
                    <td className="px-4 py-3 text-muted">{c.applies}</td>
                    <td className="px-4 py-3 text-muted">{c.formula}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>

        <Card className="mt-4 p-5">
          <h3 className="font-display font-semibold">Example cost response</h3>
          <pre className="scrollbar-thin mt-3 overflow-x-auto rounded-lg bg-surface2/60 p-4 text-[13px] leading-relaxed">
            <code>{`{
  "request_id": "req_6d2b04",
  "model": "bytedance_seedance_2",
  "status": "succeeded",
  "cost": {
    "amount_usd": 0.60,
    "currency": "USD",
    "unit": "per_video_second",
    "breakdown": "5s × $0.12 / video second"
  },
  "end_user": "cust_a2f_user_5512"
}`}</code>
          </pre>
        </Card>
      </section>

      {/* ── End-user attribution ── */}
      <section className="mt-14">
        <h2 className="font-display text-2xl font-bold tracking-tight">End-user attribution</h2>
        <p className="mt-2 max-w-3xl text-sm text-muted">
          Pass an <code className="rounded bg-surface2 px-1.5 py-0.5">end_user</code> string on any request to tag
          it with your own customer's identifier. Foundation records it on the request ledger so you can see{" "}
          <strong className="text-fg">everything that transited your account</strong> — not just calls your team
          made in the dashboard, but the traffic your customers' end-users generated through you. Filter, export,
          and reconcile per end-user in the dashboard's <em>Requests</em> view.
        </p>
        <div className="mt-4 flex flex-wrap gap-2">
          {(["image", "video", "3d", "world", "chat"] as const).map((m) => (
            <Badge key={m}>{MODALITY_LABEL[m]}</Badge>
          ))}
        </div>
      </section>
    </Container>
  );
}
