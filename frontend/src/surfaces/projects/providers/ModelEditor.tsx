import { Checkbox, Button, Input, Select } from "@/components/ui";
import { Field, RateInputs } from "./fields";
import { kindLabel, slug, type Connection, type Kind, type ModelDraft } from "./provider-onboarding";

export function ModelEditor({ model, connection, position, onChange, onRemove }: {
  model: ModelDraft;
  connection: Connection;
  position: number;
  onChange: (patch: Partial<ModelDraft>) => void;
  onRemove: () => void;
}) {
  const evidence = model.evidence;
  const update = (patch: Partial<ModelDraft>) => onChange({ ...patch, reviewed: false });
  return (
    <fieldset className="min-w-0 space-y-3 rounded-lg border border-border p-3 sm:p-4">
      <legend className="max-w-full break-words px-1 text-sm font-semibold">
        {evidence?.name || model.upstream || `Model ${position}`}
      </legend>
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="Upstream model ID">
          <Input required maxLength={160} value={model.upstream} placeholder="vendor/model"
            onChange={(event) => update({ upstream: event.target.value })} />
        </Field>
        <Field label="Gateway model alias">
          <Input maxLength={64} value={model.alias} placeholder={slug(model.upstream) || "Generated from model ID"}
            onChange={(event) => update({ alias: event.target.value })} />
        </Field>
      </div>
      {connection === "connector" && (
        <Field label="Output type"><Select value={model.kind} onChange={(event) => update({
          kind: event.target.value as Kind, amount: "", output: "",
        })}>
          {(["image", "video", "audio", "mesh"] as const).map((kind) => <option key={kind} value={kind}>{kindLabel[kind]}</option>)}
        </Select></Field>
      )}
      <RateInputs kind={model.kind} rates={{ amount: model.amount, output: model.output }} change={update} />
      <div className="grid gap-3 sm:grid-cols-2">
        {(model.kind === "image" || model.kind === "video") && (
          <Field label="Supported aspect ratios">
            <Input required value={model.ratios} placeholder="1:1, 16:9"
              onChange={(event) => update({ ratios: event.target.value })} />
          </Field>
        )}
        {(model.kind === "audio" || model.kind === "video") && (
          <Field label="Supported durations in seconds">
            <Input required value={model.durations} placeholder="5, 10"
              onChange={(event) => update({ durations: event.target.value })} />
          </Field>
        )}
        {model.kind === "video" && (
          <Field label="Resolution">
            <Input required value={model.resolution} placeholder="720p"
              onChange={(event) => update({ resolution: event.target.value })} />
          </Field>
        )}
      </div>
      {evidence && (
        <div className="space-y-2 rounded-lg bg-surface2 p-3 text-sm">
          <a href={evidence.source_url} target="_blank" rel="noreferrer" className="text-brand underline">Model documentation</a>
          {evidence.price ? (
            <p className="break-words"><a href={evidence.price.source_url} target="_blank" rel="noreferrer" className="text-brand underline">Published pricing</a>
              {" · "}{evidence.price.published}</p>
          ) : <p className="text-muted">No cited price was found. Enter the rate after checking the provider.</p>}
          <label className="flex items-start gap-2">
            <Checkbox className="mt-1"  required checked={model.reviewed}
              onChange={(event) => onChange({ reviewed: event.target.checked })} />
            <span>I checked the model and the rate I’m entering.</span>
          </label>
        </div>
      )}
      <Button type="button" size="sm" variant="ghost" onClick={onRemove}>Remove model</Button>
    </fieldset>
  );
}
