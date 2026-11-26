import { useEffect, useRef, useState } from "react";
import { ArrowDownToLine, Search } from "lucide-react";
import { Button, Input } from "@/components/ui";
import { errorMessage } from "@/lib/gateway";
import { Field } from "./fields";
import { kindLabel, providerOnboarding, type Connection, type Discovery, type DiscoveredModel } from "./provider-onboarding";

export function DiscoveryPanel({ scope, enabled, connection, source, setSource, added, capacity, onAdd }: {
  scope: import("./provider-onboarding").ProviderScope;
  enabled: boolean;
  connection: Connection;
  source: string;
  setSource: (source: string) => void;
  added: Set<string>;
  capacity: number;
  onAdd: (models: DiscoveredModel[], checkedAt: string) => void;
}) {
  const [report, setReport] = useState<Discovery>();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const request = useRef<AbortController | undefined>(undefined);
  useEffect(() => () => request.current?.abort(), []);

  async function discover() {
    if (pending) return;
    const controller = new AbortController();
    request.current = controller;
    setPending(true);
    setError("");
    setReport(undefined);
    try {
      setReport(await providerOnboarding.discover(scope, source.trim(), controller.signal));
    } catch (failure) {
      if (!controller.signal.aborted) setError(errorMessage(failure));
    } finally {
      if (!controller.signal.aborted) setPending(false);
    }
  }

  if (!enabled) return <p className="text-sm text-muted">AI discovery is unavailable on this gateway. You can enter models and rates below.</p>;
  const compatible = report?.models.filter((model) => (model.kind === "text") === (connection === "chat")) ?? [];
  const remaining = compatible.filter((model) => !added.has(model.model_id));
  return (
    <div className="space-y-3 rounded-lg border border-border p-3 sm:p-4">
      <h4 className="font-semibold">Find models and pricing</h4>
      <p className="text-sm text-muted">Search public provider documentation with AI. Review the sources and fill in missing rates before preparing a change.</p>
      <div className="flex flex-wrap items-end gap-2">
        <div className="min-w-0 flex-[1_1_15rem]">
          <Field label="Public documentation or pricing URL">
            <Input value={source} disabled={pending} maxLength={2048} placeholder="https://provider.com/docs/models"
              onChange={(event) => {
                setSource(event.target.value);
                setReport(undefined);
                setError("");
              }} />
          </Field>
        </div>
        <Button type="button" variant="outline" disabled={pending || !source.trim()} onClick={() => void discover()}>
          <Search size={15} aria-hidden="true" />{pending ? "Searching…" : "Search with AI"}
        </Button>
      </div>
      {error && <p role="alert" className="text-sm text-rose-600 dark:text-rose-300">{error}</p>}
      {pending && <p role="status" className="text-sm text-muted">Looking up public model and pricing pages…</p>}
      {report && (
        <div className="space-y-2">
          <p role="status" className="text-sm text-muted">{compatible.length
            ? `${compatible.length} sourced ${compatible.length === 1 ? "model" : "models"} for this connection. Results may be incomplete.`
            : "No sourced models matched this connection. Try a direct model page or add a model manually."}</p>
          {remaining.length > 1 && capacity > 1 && (
            <Button type="button" size="sm" variant="outline" onClick={() => onAdd(remaining, report.checked_at)}>
              Add {Math.min(remaining.length, capacity)} models
            </Button>
          )}
          {compatible.length > 0 && <ul className="max-h-80 divide-y divide-border overflow-y-auto rounded-lg border border-border">
            {compatible.map((model) => (
              <li key={model.model_id} className="flex flex-wrap items-center justify-between gap-2 p-3 text-sm">
                <div className="min-w-0 flex-1">
                  <p className="break-words font-medium">{model.name} <span className="text-muted">· {kindLabel[model.kind]}</span></p>
                  <code className="block break-all text-xs text-muted">{model.model_id}</code>
                  <a className="text-brand underline" href={model.source_url} target="_blank" rel="noreferrer">Model source</a>
                  {model.price && <span className="break-words"> · <a className="text-brand underline" href={model.price.source_url}
                    target="_blank" rel="noreferrer">{model.price.published}</a></span>}
                </div>
                <Button type="button" size="sm" variant="outline" disabled={added.has(model.model_id) || capacity <= 0}
                  onClick={() => onAdd([model], report.checked_at)}>
                  <ArrowDownToLine size={14} aria-hidden="true" />{added.has(model.model_id) ? "Added" : "Add"}
                </Button>
              </li>
            ))}
          </ul>}
        </div>
      )}
    </div>
  );
}
