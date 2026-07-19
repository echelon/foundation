import { useState } from "react";
import { Loader2, Play, RotateCcw, Sparkles } from "lucide-react";
import { Badge, Button, Card, Input, Label, Select, Slider, Textarea } from "@/components/ui";
import { mockGenerate, type GenerateResult } from "@/lib/mock-api";
import { MODALITY_LABEL, priceLabel, type Control, type Model } from "@/lib/catalog";
import { cn } from "@/lib/cn";

type Values = Record<string, unknown>;

function defaults(model: Model): Values {
  const v: Values = {};
  for (const c of model.controls) {
    if (c.type === "slider") v[c.key] = c.default;
    else if (c.type === "toggle") v[c.key] = c.default ?? false;
    else if (c.type === "select") v[c.key] = c.default ?? c.options[0]?.value;
    else if (c.type === "text" || c.type === "textarea") v[c.key] = c.default ?? "";
    else if (c.type === "seed") v[c.key] = "";
  }
  return v;
}

export function Playground({ model }: { model: Model }) {
  const [values, setValues] = useState<Values>(() => defaults(model));
  const [status, setStatus] = useState<"idle" | "running" | "done">("idle");
  const [result, setResult] = useState<GenerateResult | null>(null);

  const set = (k: string, val: unknown) => setValues((v) => ({ ...v, [k]: val }));

  const run = async () => {
    setStatus("running");
    setResult(null);
    const r = await mockGenerate(model, values);
    setResult(r);
    setStatus("done");
  };

  const reset = () => {
    setValues(defaults(model));
    setStatus("idle");
    setResult(null);
  };

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)]">
      {/* ── Controls ── */}
      <Card className="p-5">
        <div className="mb-4 flex items-center justify-between">
          <h3 className="font-display font-semibold">Inputs</h3>
          <Badge tone="brand">{MODALITY_LABEL[model.modality]}</Badge>
        </div>
        <div className="space-y-4">
          {model.controls.map((c) => (
            <ControlField key={c.key} control={c} value={values[c.key]} onChange={(val) => set(c.key, val)} />
          ))}
        </div>
        <div className="mt-6 flex items-center gap-2">
          <Button onClick={run} disabled={status === "running"} className="flex-1">
            {status === "running" ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
            {status === "running" ? "Generating…" : "Run"}
          </Button>
          <Button variant="outline" onClick={reset} disabled={status === "running"}>
            <RotateCcw className="h-4 w-4" />
          </Button>
        </div>
        <p className="mt-3 text-xs text-muted">
          Estimated {priceLabel(model.pricing[0])}. Mock run — no request leaves your browser.
        </p>
      </Card>

      {/* ── Output ── */}
      <Card className="flex flex-col overflow-hidden">
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <h3 className="font-display font-semibold">Output</h3>
          {result && <Badge tone="green">succeeded</Badge>}
        </div>
        <div className="grid flex-1 place-items-center p-6">
          {status === "idle" && (
            <div className="text-center text-muted">
              <Sparkles className="mx-auto mb-3 h-8 w-8 text-brand/60" />
              <p className="text-sm">Set your inputs and hit Run to preview a generation.</p>
            </div>
          )}
          {status === "running" && (
            <div className="w-full">
              <div className="aspect-video w-full animate-pulse rounded-xl bg-surface2" />
              <p className="mt-4 text-center text-sm text-muted">Generating a mock {MODALITY_LABEL[model.modality].toLowerCase()}…</p>
            </div>
          )}
          {status === "done" && result && (
            <div className="w-full">
              <div className="relative grid aspect-video w-full place-items-center overflow-hidden rounded-xl brand-gradient">
                <div className="absolute inset-0 grid-bg opacity-30" />
                <div className="relative text-center text-white">
                  <p className="text-5xl">✦</p>
                  <p className="mt-2 text-sm opacity-80">Mock {MODALITY_LABEL[model.modality]} preview</p>
                </div>
              </div>
              <div className="mt-4 grid grid-cols-3 gap-3 text-sm">
                <Stat label="Request" value={result.requestId} mono />
                <Stat label="Latency" value={`${(result.latencyMs / 1000).toFixed(1)}s`} />
                <Stat label="Cost" value={`$${result.costUsd.toFixed(4)}`} />
              </div>
              <p className="mt-3 rounded-lg bg-surface2/60 px-3 py-2 text-xs text-muted">
                Cost breakdown: {result.costBreakdown}
              </p>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}

function Stat({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="rounded-lg bg-surface2/60 px-3 py-2">
      <div className="text-xs text-muted">{label}</div>
      <div className={cn("truncate font-medium text-fg", mono && "font-mono text-[13px]")}>{value}</div>
    </div>
  );
}

function ControlField({ control, value, onChange }: { control: Control; value: unknown; onChange: (v: unknown) => void }) {
  switch (control.type) {
    case "text":
      return (
        <div>
          <Label hint={control.hint}>{control.label}</Label>
          <Input placeholder={control.placeholder} value={(value as string) ?? ""} onChange={(e) => onChange(e.target.value)} />
        </div>
      );
    case "textarea":
      return (
        <div>
          <Label hint={control.hint}>{control.label}</Label>
          <Textarea rows={3} placeholder={control.placeholder} value={(value as string) ?? ""} onChange={(e) => onChange(e.target.value)} />
        </div>
      );
    case "select":
      return (
        <div>
          <Label hint={control.hint}>{control.label}</Label>
          <Select value={(value as string) ?? ""} onChange={(e) => onChange(e.target.value)}>
            {control.options.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </Select>
        </div>
      );
    case "slider":
      return (
        <div>
          <Label hint={String(value)}>{control.label}</Label>
          <Slider value={Number(value)} min={control.min} max={control.max} step={control.step} onChange={onChange} />
        </div>
      );
    case "toggle":
      return (
        <label className="flex items-center justify-between">
          <span className="text-sm font-medium text-fg">{control.label}</span>
          <button
            type="button"
            onClick={() => onChange(!value)}
            className={cn("relative h-6 w-11 rounded-full transition", value ? "brand-gradient" : "bg-surface2")}
          >
            <span className={cn("absolute top-0.5 h-5 w-5 rounded-full bg-white transition", value ? "left-[1.375rem]" : "left-0.5")} />
          </button>
        </label>
      );
    case "seed":
      return (
        <div>
          <Label hint={control.hint}>{control.label}</Label>
          <Input type="number" placeholder="random" value={(value as string) ?? ""} onChange={(e) => onChange(e.target.value)} />
        </div>
      );
  }
}
