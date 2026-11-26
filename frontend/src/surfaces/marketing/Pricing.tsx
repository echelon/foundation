import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { Button, Card, Container, Input, Select } from "@/components/ui";
import { comparisonApi, providerLabel, usd, type Catalog, type Offer } from "@/surfaces/comparison/api";
import { catalogModelName, modalityNames } from "@/surfaces/requests/modelOptions";
import { CatalogUpdates } from "./CatalogUpdates";
import { PricingEstimate } from "./PricingEstimate";
import { ReferencePricing } from "./ReferencePricing";

type LoadState = { catalog: Catalog } | { failed: true };
const cell = "px-3 py-3 align-top sm:px-5";
const field = "block space-y-1.5 text-sm font-medium";
const rateTime = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

export default function Pricing() {
  const [params] = useSearchParams();
  const gateway = params.get("catalog") === "gateway";
  const link = "border-b-2 px-3 py-2 text-sm font-semibold hover:bg-surface2";
  return (
    <Container className="space-y-7 pb-14 pt-8 sm:pt-12">
      <header className="border-b border-border pb-5">
        <h1 className="font-display text-4xl font-extrabold tracking-[-0.05em] sm:text-6xl">Provider pricing</h1>
        <nav aria-label="Pricing catalogs" className="mt-4 flex flex-wrap gap-2">
          <Link to="/pricing" aria-current={gateway ? undefined : "page"} className={`${link} ${gateway ? "border-transparent text-muted" : "border-brand text-fg"}`}>ArtCraft catalog</Link>
          <Link to="/pricing?catalog=gateway" aria-current={gateway ? "page" : undefined} className={`${link} ${gateway ? "border-brand text-fg" : "border-transparent text-muted"}`}>This gateway</Link>
        </nav>
      </header>
      {gateway ? <GatewayPricing /> : <ReferencePricing />}
    </Container>
  );
}

function GatewayPricing() {
  const [loaded, setLoaded] = useState<LoadState>();
  const [retry, setRetry] = useState(0);
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState("");
  const [provider, setProvider] = useState("");
  const [updating, setUpdating] = useState(false);
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    const controller = new AbortController();
    comparisonApi.catalog(controller.signal).then((catalog) => {
      if (!controller.signal.aborted) setLoaded({ catalog });
    }).catch(() => {
      if (!controller.signal.aborted) setLoaded({ failed: true });
    });
    return () => controller.abort();
  }, [retry]);

  function reload() {
    setLoaded(undefined);
    setUpdating(false);
    setRetry((attempt) => attempt + 1);
  }

  const catalog = loaded && "catalog" in loaded ? loaded.catalog : undefined;
  useEffect(() => {
    const deadlines = Object.values(catalog?.rate_valid_until ?? {}).map((value) => Date.parse(value));
    const next = Math.min(...deadlines.filter((deadline) => deadline > now));
    if (!Number.isFinite(next)) return;
    const timer = window.setTimeout(() => setNow(Date.now()), Math.max(0, next - Date.now()) + 1);
    return () => window.clearTimeout(timer);
  }, [catalog, now]);

  const offers = catalog?.offers ?? [];
  const hasImportedOffers = offers.some((offer) => offer.artcraft_reference);
  const providers = [...new Set(offers.map((offer) => offer.provider))]
    .sort((a, b) => providerLabel(a).localeCompare(providerLabel(b)));
  const query = search.trim().toLowerCase();
  const matches = offers.filter((offer) => {
    if (kind && offer.pricing.kind !== kind) return false;
    if (provider && offer.provider !== provider) return false;
    const terms = [
      offer.model, catalogModelName(offer.model), offer.id,
      offer.provider, providerLabel(offer.provider), offer.artcraft_reference?.offer_id,
    ];
    return terms.some((term) => term?.toLowerCase().includes(query));
  }).sort((a, b) => a.model.localeCompare(b.model)
    || providerLabel(a.provider).localeCompare(providerLabel(b.provider)) || a.id.localeCompare(b.id));

  return (
    <div className="space-y-6">
      <div>
        <p className="text-sm text-muted">Public rates and model capabilities configured by this gateway’s operator.</p>
        {offers.length > 0 && <a href="#estimate" className="mt-2 inline-block font-mono text-sm font-semibold text-brand hover:underline">Compare a workload ↓</a>}
      </div>

      {!loaded && <p role="status" className="text-sm text-muted">Loading gateway pricing…</p>}
      {loaded && "failed" in loaded && (
        <div className="flex flex-wrap items-center gap-3">
          <p role="alert" className="text-sm text-muted">Gateway pricing is unavailable.</p>
          <Button size="sm" variant="outline" onClick={reload}>Retry</Button>
        </div>
      )}
      {catalog && (offers.length ? (
        <>
          <section className="space-y-4" aria-labelledby="configured-rates">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h2 id="configured-rates" className="font-display text-2xl font-bold tracking-tight">Configured rates</h2>
              <div className="flex flex-wrap gap-2">
                {hasImportedOffers && <Button size="sm" variant="outline" aria-expanded={updating}
                  aria-controls="catalog-updates" onClick={() => setUpdating((open) => !open)}>Check preset updates</Button>}
                <Button size="sm" variant="outline" onClick={reload}>Refresh rates</Button>
              </div>
            </div>
            {hasImportedOffers && <div id="catalog-updates" hidden={!updating}>
              {updating && <CatalogUpdates offers={offers} />}
            </div>}
            <div className="grid gap-3 sm:grid-cols-[minmax(0,2fr)_1fr_1fr]">
              <label className={field}>Search rates<Input type="search" value={search} onChange={(event) => setSearch(event.target.value)} /></label>
              <div className={field}>
                <label htmlFor="pricing-modality">Modality</label>
                <Select id="pricing-modality" value={kind} onChange={(event) => setKind(event.target.value)}>
                  <option value="">All modalities</option>
                  {Object.entries(modalityNames).map(([id, name]) => <option key={id} value={id}>{name}</option>)}
                </Select>
              </div>
              <div className={field}>
                <label htmlFor="pricing-provider">Provider</label>
                <Select id="pricing-provider" value={provider} onChange={(event) => setProvider(event.target.value)}>
                  <option value="">All providers</option>
                  {providers.map((id) => <option key={id} value={id}>{providerLabel(id)}</option>)}
                </Select>
              </div>
            </div>
            <Card className="overflow-hidden">
              <table aria-labelledby="configured-rates" className="w-full table-fixed text-left text-sm">
                <thead className="border-b border-border bg-surface2/60 text-muted">
                  <tr>
                    <th scope="col" className={`${cell} w-[52%] font-medium sm:w-[60%]`}>Model and provider</th>
                    <th scope="col" className={`${cell} font-medium`}>Rate</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {matches.length ? matches.map((offer) => <RateRow key={offer.id} offer={offer} validUntil={catalog.rate_valid_until?.[offer.id]} now={now} />)
                    : <tr><td colSpan={2} className={`${cell} text-muted`}>No matching offers.</td></tr>}
                </tbody>
              </table>
            </Card>
          </section>
          <PricingEstimate />
        </>
      ) : <p role="status" className="text-sm text-muted">This gateway has no configured offers.</p>)}
    </div>
  );
}

function RateRow({ offer, validUntil, now }: { offer: Offer; validUntil?: string; now: number }) {
  const expiry = validUntil ? Date.parse(validUntil) : NaN;
  const expired = expiry <= now;
  const { model, pricing, artcraft_reference: preset } = offer;
  const name = catalogModelName(model);
  const rate = (value: number, unit: string) => (
    <div><span className="font-mono font-medium tabular-nums">{usd(value)}</span> <span className="text-muted">{unit}</span></div>
  );
  let rates;
  let capability;
  switch (pricing.kind) {
    case "image":
      rates = rate(pricing.usd_nanos_per_image, "/ image");
      capability = pricing.aspect_ratios.join(", ");
      break;
    case "video":
      rates = rate(pricing.usd_nanos_per_second, "/ second");
      capability = [pricing.resolution, pricing.durations_seconds.map((value) => `${value}s`).join(", "),
        pricing.aspect_ratios.join(", ")].join(" · ");
      break;
    case "audio":
      rates = rate(pricing.usd_nanos_per_second, "/ second");
      capability = `${pricing.format.toUpperCase()} · ${pricing.durations_seconds.map((value) => `${value}s`).join(", ")}`;
      break;
    case "mesh":
      rates = rate(pricing.usd_nanos_per_mesh, "/ mesh");
      capability = pricing.format.toUpperCase();
      break;
    case "text":
      rates = <>{rate(pricing.usd_nanos_per_1000_input_tokens, "/ 1K input tokens")}{rate(pricing.usd_nanos_per_1000_output_tokens, "/ 1K output tokens")}</>;
      break;
  }
  return (
    <tr>
      <th scope="row" className={`${cell} break-words font-medium`}>
        {name && <span className="block">{name}</span>}
        <code className="block break-all text-xs font-normal">{model}</code>
        <span className="mt-1 block text-xs font-normal text-muted">
          {providerLabel(offer.provider)} · {modalityNames[pricing.kind]}{offer.adapter === "mock" && " · Fixture"}
        </span>
        <span className="block break-all text-xs font-normal text-muted">{offer.id}</span>
        {preset && (
          <details className="mt-2 text-xs font-normal">
            <summary className="cursor-pointer font-medium text-brand">ArtCraft preset</summary>
            <dl className="mt-2 space-y-1 text-muted">
              <div><dt className="font-medium">Offer ID</dt><dd className="break-all font-mono">{preset.offer_id}</dd></div>
              <div><dt className="font-medium">Reference revision</dt><dd className="break-all font-mono">{preset.revision}</dd></div>
              <div><dt className="font-medium">Snapshot SHA-256</dt><dd className="break-all font-mono">{preset.sha256}</dd></div>
              {preset.public_key_sha256 && (
                <div><dt className="font-medium">Signer SHA-256</dt><dd className="break-all font-mono">{preset.public_key_sha256}</dd></div>
              )}
            </dl>
          </details>
        )}
      </th>
      <td className={`${cell} break-words`}>
        {rates}
        {capability && <p className="mt-1 text-xs text-muted">{capability}</p>}
        {Number.isFinite(expiry) && (
          <p className={`mt-1 text-xs ${expired ? "font-semibold text-brand" : "text-muted"}`}>
            {expired ? "Rate expired" : "Rate valid until"} <time dateTime={validUntil}>{rateTime.format(expiry)}</time>
          </p>
        )}
      </td>
    </tr>
  );
}
