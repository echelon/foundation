import { Link } from "react-router-dom";
import { ArrowRight, Boxes, Check, Globe, Image as ImageIcon, MessageSquare, Sparkles, Video, Zap } from "lucide-react";
import { Badge, Button, Card, Container } from "@/components/ui";
import { ModelCard } from "@/components/ModelCard";
import { MODELS, MODALITY_LABEL, type Modality } from "@/lib/catalog";

const MODALITIES: { modality: Modality; icon: typeof ImageIcon; blurb: string }[] = [
  { modality: "image", icon: ImageIcon, blurb: "Photoreal, editable, up to 4K." },
  { modality: "video", icon: Video, blurb: "Cinematic clips with camera control." },
  { modality: "3d", icon: Boxes, blurb: "Watertight meshes with PBR textures." },
  { modality: "world", icon: Globe, blurb: "Explorable 3D worlds as splats." },
  { modality: "chat", icon: MessageSquare, blurb: "Frontier LLMs, one endpoint." },
];

const featured = ["google_nano_banana_2", "openai_gpt_image_v2", "bytedance_seedance_v2", "worldlabs_marble_v1_1", "tencent_hunyuan3d_v2_5", "anthropic_claude_opus_4_8"]
  .map((s) => MODELS.find((m) => m.slug === s)!)
  .filter(Boolean);

export default function Landing() {
  return (
    <>
      {/* ── Hero ── */}
      <section className="relative overflow-hidden">
        <div className="grid-bg absolute inset-0 opacity-60" />
        <div className="pointer-events-none absolute -top-40 left-1/2 h-96 w-[42rem] -translate-x-1/2 rounded-full brand-gradient opacity-20 blur-3xl" />
        <Container className="relative py-24 text-center sm:py-32">
          <Badge tone="brand" className="mx-auto mb-6">
            <Sparkles className="h-3 w-3" /> Nano Banana 2 · GPT Image 2 · Seedance 2.0 — live
          </Badge>
          <h1 className="mx-auto max-w-4xl font-display text-5xl font-extrabold tracking-tight sm:text-6xl">
            Every generative model,{" "}
            <span className="brand-text">one clean API.</span>
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted">
            Foundation is the accessible, well-organized gateway to the best image, video, 3D, and language
            models. Predictable pricing, a great playground, and one key for everything.
          </p>
          <div className="mt-9 flex flex-wrap items-center justify-center gap-3">
            <Link to="/dashboard">
              <Button size="lg">
                Start building <ArrowRight className="h-4 w-4" />
              </Button>
            </Link>
            <Link to="/models">
              <Button size="lg" variant="outline">
                Browse models
              </Button>
            </Link>
          </div>
          <p className="mt-4 text-sm text-muted">No credit card to explore · pay only for what you generate</p>
        </Container>
      </section>

      {/* ── Modalities ── */}
      <Container className="py-16">
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
          {MODALITIES.map(({ modality, icon: Icon, blurb }) => (
            <Link key={modality} to={`/models?modality=${modality}`}>
              <Card className="h-full p-5 transition hover:border-brand/50 hover:shadow-glow">
                <div className="mb-3 grid h-10 w-10 place-items-center rounded-lg bg-brand/12 text-brand">
                  <Icon className="h-5 w-5" />
                </div>
                <h3 className="font-display font-semibold">{MODALITY_LABEL[modality]}</h3>
                <p className="mt-1 text-sm text-muted">{blurb}</p>
              </Card>
            </Link>
          ))}
        </div>
      </Container>

      {/* ── Code sample ── */}
      <Container className="py-16">
        <div className="grid items-center gap-10 lg:grid-cols-2">
          <div>
            <h2 className="font-display text-3xl font-bold tracking-tight">One request, any model</h2>
            <p className="mt-4 text-muted">
              Call a model directly by its catalog slug, or hit a modality endpoint and let Foundation route to
              the best available model. Every generate endpoint has a <code className="rounded bg-surface2 px-1.5 py-0.5 text-sm">/cost</code>{" "}
              twin so you can price a request before you run it.
            </p>
            <ul className="mt-6 space-y-2 text-sm">
              {["Model-specific & modality endpoints", "OpenAI-compatible /v1/chat/completions", "Per-request cost breakdowns", "Trace usage down to your end-users"].map((f) => (
                <li key={f} className="flex items-center gap-2 text-fg">
                  <Check className="h-4 w-4 text-brand" /> {f}
                </li>
              ))}
            </ul>
            <Link to="/docs" className="mt-6 inline-flex">
              <Button variant="outline">Read the docs</Button>
            </Link>
          </div>
          <Card className="overflow-hidden">
            <div className="flex items-center gap-1.5 border-b border-border px-4 py-3">
              <span className="h-3 w-3 rounded-full bg-rose-400/70" />
              <span className="h-3 w-3 rounded-full bg-amber-400/70" />
              <span className="h-3 w-3 rounded-full bg-emerald-400/70" />
              <span className="ml-3 text-xs text-muted">generate.sh</span>
            </div>
            <pre className="scrollbar-thin overflow-x-auto p-5 text-[13px] leading-relaxed">
              <code>{`curl https://api.foundation.ai/v1/any/image \\
  -H "Authorization: Bearer $FOUNDATION_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "model": "google_nano_banana_2",
    "prompt": "neon-lit ramen shop, rain, cinematic",
    "aspect_ratio": "16:9",
    "num_images": 1,
    "end_user": "cust_a2f_user_5512"
  }'`}</code>
            </pre>
          </Card>
        </div>
      </Container>

      {/* ── Featured models ── */}
      <Container className="py-16">
        <div className="mb-6 flex items-end justify-between">
          <div>
            <h2 className="font-display text-3xl font-bold tracking-tight">Featured models</h2>
            <p className="mt-2 text-muted">A slice of the catalog. {MODELS.length} models and counting.</p>
          </div>
          <Link to="/models" className="hidden sm:block">
            <Button variant="ghost">
              View all <ArrowRight className="h-4 w-4" />
            </Button>
          </Link>
        </div>
        <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
          {featured.map((m) => (
            <ModelCard key={m.slug} model={m} />
          ))}
        </div>
      </Container>

      {/* ── CTA ── */}
      <Container className="py-20">
        <Card className="relative overflow-hidden p-12 text-center">
          <div className="pointer-events-none absolute inset-0 brand-gradient opacity-[0.08]" />
          <Zap className="relative mx-auto mb-4 h-8 w-8 text-brand" />
          <h2 className="relative font-display text-3xl font-bold tracking-tight">Ship your first generation today</h2>
          <p className="relative mx-auto mt-3 max-w-xl text-muted">
            Spin up a key, try any model in the playground, and go to production on the same API.
          </p>
          <Link to="/signup" className="relative mt-7 inline-flex">
            <Button size="lg">
              Create an account <ArrowRight className="h-4 w-4" />
            </Button>
          </Link>
        </Card>
      </Container>
    </>
  );
}
