import { Link } from "react-router-dom";
import { Check } from "lucide-react";
import { Badge, Button, Card, Container } from "@/components/ui";
import { MODELS, MODALITY_LABEL, priceLabel, type Modality } from "@/lib/catalog";

const ORDER: Modality[] = ["image", "video", "3d", "world", "chat"];

export default function Pricing() {
  return (
    <>
      <Container className="py-16 text-center">
        <h1 className="font-display text-4xl font-extrabold tracking-tight sm:text-5xl">
          Simple, <span className="brand-text">usage-based</span> pricing
        </h1>
        <p className="mx-auto mt-4 max-w-2xl text-muted">
          You prepay credits and pay only for what you generate. No seats, no minimums. Every generate endpoint
          has a <code className="rounded bg-surface2 px-1.5 py-0.5 text-sm">/cost</code> twin so you always know
          the price before you run a request.
        </p>
      </Container>

      {/* ── How billing works ── */}
      <Container className="pb-8">
        <div className="grid gap-4 md:grid-cols-3">
          <HowCard title="Prepaid credits" body="Top up a balance in USD. Requests draw down from it; enable auto-reload so you never run dry mid-batch." />
          <HowCard title="Per-unit metering" body="Each modality meters differently — per image, per video-second, per mesh, per world, or per million tokens." />
          <HowCard title="Tax included at checkout" body="Applicable sales tax is calculated and added on every top-up and invoice. Invoices are numbered gaplessly." />
        </div>
      </Container>

      {/* ── Price tables per modality ── */}
      <Container className="space-y-10 py-8">
        {ORDER.map((modality) => {
          const models = MODELS.filter((m) => m.modality === modality);
          if (!models.length) return null;
          return (
            <div key={modality}>
              <div className="mb-3 flex items-center gap-2">
                <h2 className="font-display text-2xl font-bold tracking-tight">{MODALITY_LABEL[modality]}</h2>
                <Badge>{models.length} models</Badge>
              </div>
              <Card className="overflow-hidden">
                <div className="scrollbar-thin overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead className="border-b border-border bg-surface2/50 text-left text-muted">
                      <tr>
                        <th className="px-4 py-3 font-medium">Model</th>
                        <th className="px-4 py-3 font-medium">Provider</th>
                        <th className="px-4 py-3 font-medium">Unit price</th>
                        <th className="px-4 py-3 font-medium">Notes</th>
                      </tr>
                    </thead>
                    <tbody>
                      {models.map((m) => (
                        <tr key={m.slug} className="border-b border-border/60 last:border-0">
                          <td className="px-4 py-3">
                            <Link to={`/models/${m.slug}`} className="font-medium text-fg hover:text-brand">
                              {m.name}
                            </Link>
                          </td>
                          <td className="px-4 py-3 text-muted">{m.provider}</td>
                          <td className="px-4 py-3">
                            <div className="space-y-0.5">
                              {m.pricing.map((p, i) => (
                                <div key={i} className="font-medium text-fg">
                                  {priceLabel(p)}
                                </div>
                              ))}
                            </div>
                          </td>
                          <td className="px-4 py-3 text-muted">
                            {m.pricing.map((p) => p.note).filter(Boolean).join(" · ") || "—"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </Card>
            </div>
          );
        })}
      </Container>

      {/* ── Enterprise ── */}
      <Container className="py-12">
        <Card className="grid items-center gap-8 p-8 md:grid-cols-2">
          <div>
            <h2 className="font-display text-2xl font-bold tracking-tight">Scaling past self-serve?</h2>
            <p className="mt-3 text-muted">
              Enterprise plans add committed-use discounts, invoicing terms, dedicated capacity, SSO, and
              per-end-user usage exports so you can bill your own customers.
            </p>
            <div className="mt-5 flex gap-3">
              <Button>Talk to sales</Button>
              <Link to="/docs">
                <Button variant="outline">Read the docs</Button>
              </Link>
            </div>
          </div>
          <ul className="space-y-3 text-sm">
            {["Committed-use discounts", "Net terms & consolidated invoicing", "SSO / SAML", "Dedicated throughput", "End-user usage exports", "Priority support"].map((f) => (
              <li key={f} className="flex items-center gap-2 text-fg">
                <Check className="h-4 w-4 text-brand" /> {f}
              </li>
            ))}
          </ul>
        </Card>
      </Container>
    </>
  );
}

function HowCard({ title, body }: { title: string; body: string }) {
  return (
    <Card className="p-5">
      <h3 className="font-display font-semibold">{title}</h3>
      <p className="mt-2 text-sm text-muted">{body}</p>
    </Card>
  );
}
