import { useEffect, useState, type FormEvent } from "react";
import { Button, Card, Input } from "@/components/ui";
import { downloadBlob } from "@/lib/download";
import type { Offer } from "@/surfaces/comparison/api";
import { canPrepare, downloadReferenceSnapshot, loadReferenceSnapshot, referenceCatalogFile, type ReferenceSnapshot } from "./referenceCatalog";

const sha256 = /^[0-9a-f]{64}$/i;
const selectionFile = "catalog-updates.json";
type LoadState = { snapshot: ReferenceSnapshot } | { failed: true };

function updateRows(offers: Offer[], snapshot: ReferenceSnapshot) {
  const targets = new Map(snapshot.catalog.offers.map((offer) => [offer.id, offer]));
  return offers.flatMap((offer) => {
    const installed = offer.artcraft_reference;
    if (!installed) return [];
    const target = targets.get(installed.offer_id);
    const adapter = target?.plugin === "openrouter" ? "openai_compatible_chat" : "connector_queue";
    let unavailable;
    if (!sha256.test(installed.sha256)) {
      unavailable = "The installed preset is missing a valid checksum.";
    } else if (installed.sha256.toLowerCase() === snapshot.checksum) {
      unavailable = "Already on this snapshot.";
    } else if (installed.revision === snapshot.catalog.revision) {
      unavailable = "This snapshot has different bytes under the installed revision.";
    } else if (!target) {
      unavailable = "The preset is absent from this snapshot.";
    } else if (!canPrepare(target) || target.plugin !== offer.provider || adapter !== offer.adapter
      || target.kind !== offer.pricing.kind || (offer.upstream_model && target.upstream_model !== offer.upstream_model)) {
      unavailable = "The preset’s routing details changed. Review this offer manually.";
    }
    return [{ offer, installed, unavailable }];
  });
}

export function CatalogUpdates({ offers }: { offers: Offer[] }) {
  const [loaded, setLoaded] = useState<LoadState>();
  const [retry, setRetry] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    loadReferenceSnapshot(controller.signal).then((snapshot) => {
      if (!controller.signal.aborted) setLoaded({ snapshot });
    }).catch(() => {
      if (!controller.signal.aborted) setLoaded({ failed: true });
    });
    return () => controller.abort();
  }, [retry]);

  return (
    <Card className="min-w-0 space-y-4 p-4 sm:p-5">
      <h3 className="font-display text-xl font-semibold">Prepare preset updates</h3>
      {!loaded ? <p role="status" className="text-sm text-muted">Checking the ArtCraft catalog…</p>
        : "failed" in loaded ? (
          <div className="flex flex-wrap items-center gap-3">
            <p role="alert" className="text-sm text-muted">The ArtCraft catalog is unavailable.</p>
            <Button size="sm" variant="outline" onClick={() => {
              setLoaded(undefined);
              setRetry((attempt) => attempt + 1);
            }}>Retry catalog</Button>
          </div>
        ) : !loaded.snapshot.checksum ? (
          <p className="text-sm text-muted">Open Pricing over HTTPS or localhost to pin the catalog checksum.</p>
        ) : <UpdateSelection offers={offers} snapshot={loaded.snapshot} />}
    </Card>
  );
}

function UpdateSelection({ offers, snapshot }: { offers: Offer[]; snapshot: ReferenceSnapshot }) {
  const [selected, setSelected] = useState(() => new Set<string>());
  const [key, setKey] = useState("");
  const [error, setError] = useState("");
  const rows = updateRows(offers, snapshot);
  const changes = rows.filter((row) => !row.unavailable && selected.has(row.offer.id));
  const installedKeys = new Set(changes.flatMap(({ installed }) => installed.public_key_sha256 ? [installed.public_key_sha256] : []));
  const requiresSignature = installedKeys.size > 0;
  const input = requiresSignature ? "--bundle artcraft.next.bundle.json --public-key artcraft-public.der" : `--reference ${referenceCatalogFile}`;

  function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!snapshot.checksum || !changes.length) return;
    const keyPin = requiresSignature ? key.trim() || (installedKeys.size === 1 ? [...installedKeys][0] : "") : undefined;
    if (requiresSignature && !sha256.test(keyPin ?? "")) {
      setError("Enter the trusted publisher key SHA-256 as 64 hexadecimal characters.");
      return;
    }
    const selection = {
      version: 1,
      reference_revision: snapshot.catalog.revision,
      reference_sha256: snapshot.checksum,
      reference_public_key_sha256: keyPin?.toLowerCase(),
      offers: changes.map(({ offer, installed }) => ({
        id: offer.id,
        reference: installed.offer_id,
        update_from_sha256: installed.sha256,
      })),
    };
    downloadBlob(new Blob([JSON.stringify(selection, null, 2) + "\n"], { type: "application/json" }), selectionFile);
    setError("");
  }

  function toggle(id: string, checked: boolean) {
    setSelected((previous) => {
      const next = new Set(previous);
      if (checked) next.add(id);
      else next.delete(id);
      return next;
    });
    setError("");
  }

  return (
    <form className="space-y-4" onSubmit={save}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="min-w-0 break-all text-sm text-muted">Target revision <code>{snapshot.catalog.revision}</code></p>
        <Button type="button" size="sm" variant="outline" onClick={() => downloadReferenceSnapshot(snapshot)}>Download target JSON</Button>
      </div>
      <fieldset className="min-w-0">
        <legend className="mb-2 text-sm font-medium">Imported offers</legend>
        <div className="divide-y divide-border rounded-lg border border-border">
          {rows.map(({ offer, installed, unavailable }) => (
            <label key={offer.id} className="flex items-start gap-3 p-3 text-sm">
              <input type="checkbox" className="mt-0.5 h-4 w-4 shrink-0 accent-brand" disabled={Boolean(unavailable)}
                checked={selected.has(offer.id)} onChange={(event) => toggle(offer.id, event.target.checked)} />
              <span className="min-w-0">
                <span className="block break-all font-medium">{offer.id}</span>
                <span className="block break-all text-xs text-muted">{installed.offer_id} · {installed.revision}</span>
                {unavailable && <span className="mt-1 block text-xs text-muted">{unavailable}</span>}
              </span>
            </label>
          ))}
        </div>
      </fieldset>
      {requiresSignature && (
        <label className="block max-w-lg space-y-1.5 text-sm font-medium">
          <span>Publisher key SHA-256</span>
          <Input value={key} maxLength={64} autoComplete="off" spellCheck={false}
            onChange={(event) => { setKey(event.target.value); setError(""); }} />
          <span className="block text-xs font-normal text-muted">{installedKeys.size === 1
            ? "Leave blank to use the installed key. Enter the trusted new fingerprint for a rotation."
            : "Enter the trusted fingerprint for the target snapshot. The selected offers use different keys."}</span>
        </label>
      )}
      {error && <p role="alert" className="text-sm text-rose-700 dark:text-rose-300">{error}</p>}
      <Button type="submit" disabled={!changes.length}>Download update selection</Button>
      {changes.length > 0 && (
        <div className="space-y-2 text-sm text-muted">
          <p>The gateway CLI checks compatibility, then reviews the candidate for changes to the selected presets.
            {requiresSignature && " Supply a verified bundle for the target snapshot."}</p>
          <pre className="whitespace-pre-wrap break-words rounded-lg bg-surface2 p-3 text-xs">
            <code>{`api-gateway catalog prepare --base catalog.json ${input} --selection ${selectionFile} > catalog.next.json &&
api-gateway catalog review --base catalog.json --candidate catalog.next.json --require-preset-only`}</code>
          </pre>
        </div>
      )}
    </form>
  );
}
