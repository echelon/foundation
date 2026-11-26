import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Button, Card, Input, Select } from "@/components/ui";
import { OfferComparison } from "@/surfaces/comparison/OfferComparison";
import { comparisonApi, errorMessage, type ModelCatalog, type Quote, type QuoteInput } from "@/surfaces/comparison/api";
import { catalogModelName, modalityNames, modelChoices, modelPresets, presetLabels } from "@/surfaces/requests/modelOptions";

const field = "block space-y-1.5 text-sm font-medium";
type Evaluation = { input: QuoteInput } & ({ quote: Quote } | { error: string });

export function PricingEstimate() {
  const [catalog, setCatalog] = useState<ModelCatalog | null>(null);
  const [catalogError, setCatalogError] = useState(false);
  const [retry, setRetry] = useState(0);
  const [modelKey, setModelKey] = useState("");
  const [input, setInput] = useState<QuoteInput>();
  const [evaluation, setEvaluation] = useState<Evaluation>();

  useEffect(() => {
    const controller = new AbortController();
    comparisonApi.models(controller.signal).then((result) => {
      if (!controller.signal.aborted) setCatalog(result);
    }).catch(() => {
      if (!controller.signal.aborted) setCatalogError(true);
    });
    return () => controller.abort();
  }, [retry]);
  useEffect(() => {
    if (!input) return;
    const controller = new AbortController();
    comparisonApi.quote(input, controller.signal).then((quote) => {
      if (!controller.signal.aborted) setEvaluation({ input, quote });
    }).catch((error) => {
      if (!controller.signal.aborted) setEvaluation({ input, error: errorMessage(error) });
    });
    return () => controller.abort();
  }, [input]);

  const choices = useMemo(() => modelChoices(catalog), [catalog]);
  const selected = choices.find(([key]) => key === modelKey) ?? choices[0];
  const choice = selected?.[1];
  const presets = useMemo(() => modelPresets(choice), [choice]);

  function estimate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!choice) return;
    const fields = new FormData(event.currentTarget);
    let workload: QuoteInput["workload"] | undefined;
    switch (choice.kind) {
      case "image":
        workload = { kind: "image", count: Number(fields.get("count")), aspect_ratio: String(fields.get("ratio")), max_relative_error_bps: 0 };
        break;
      case "text":
        workload = { kind: "text", max_input_tokens: Number(fields.get("input")), max_output_tokens: Number(fields.get("output")) };
        break;
      default:
        workload = presets.find(([key]) => key === fields.get("preset"))?.[1];
    }
    if (!workload) return;
    setInput({ model: choice.model, provider: null, workload, max_total_usd_nanos: null });
  }

  const result = evaluation?.input === input ? evaluation : undefined;
  const loading = !!input && !result;
  const quote = result && "quote" in result ? result.quote : null;

  return (
    <section id="estimate" aria-labelledby="estimate-heading" className="space-y-4">
      <header>
        <h2 id="estimate-heading" className="font-display text-2xl font-bold tracking-tight">Compare a workload</h2>
        <p className="mt-1 text-sm text-muted">Get a read-only estimate for the inputs below.</p>
      </header>
      {catalogError && (
        <div className="flex flex-wrap items-center gap-3">
          <p role="alert" className="text-sm text-muted">Model capabilities are unavailable.</p>
          <Button size="sm" variant="outline" onClick={() => { setCatalogError(false); setRetry(retry + 1); }}>Retry models</Button>
        </div>
      )}
      {!catalog && !catalogError && <p role="status" className="text-sm text-muted">Loading model capabilities…</p>}
      {catalog && !choice && <p role="status" className="text-sm text-muted">No models are available to quote.</p>}
      {choice && (
        <div className="grid items-start gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.6fr)]">
          <Card className="p-4 sm:p-5">
            <form className="space-y-4" onSubmit={estimate} onChange={() => setInput(undefined)}>
              <div className={field}>
                <label htmlFor="estimate-model">Model</label>
                <Select id="estimate-model" value={selected[0]} onChange={(event) => setModelKey(event.target.value)}>
                  {choices.map(([key, { kind, model }]) => <option key={key} value={key}>{catalogModelName(model) ?? model} · {modalityNames[kind]}</option>)}
                </Select>
              </div>
              <div key={selected[0]} className="grid gap-3 min-[375px]:grid-cols-2">
                {choice.kind === "image" && (
                  <>
                    <div className={field}>
                      <label htmlFor="estimate-ratio">Aspect ratio</label>
                      <Select id="estimate-ratio" name="ratio" required>{choice.aspectRatios.map((ratio) => <option key={ratio}>{ratio}</option>)}</Select>
                    </div>
                    <label className={field}>Image count<Input name="count" type="number" min={1} max={8} defaultValue={1} required /></label>
                  </>
                )}
                {choice.kind === "text" && (
                  <>
                    <label className={field}>Max input tokens<Input name="input" type="number" min={1} max={128000} defaultValue={1000} required /></label>
                    <label className={field}>Max output tokens<Input name="output" type="number" min={1} max={64000} defaultValue={500} required /></label>
                  </>
                )}
                {choice.kind !== "text" && choice.kind !== "image" && (
                  <div className={`${field} col-span-full`}>
                    <label htmlFor="estimate-preset">{presetLabels[choice.kind]}</label>
                    <Select id="estimate-preset" name="preset" required>{presets.map(([label]) => <option key={label}>{label}</option>)}</Select>
                  </div>
                )}
              </div>
              <Button type="submit" disabled={loading}>Get estimate</Button>
              {result && "error" in result && <p role="alert" className="text-sm text-muted">{result.error}</p>}
            </form>
          </Card>
          <OfferComparison quote={quote} loading={loading} publicEstimate />
        </div>
      )}
    </section>
  );
}
