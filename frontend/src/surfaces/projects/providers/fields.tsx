import type { ReactNode } from "react";
import { Input } from "@/components/ui";
import { rateUnit, type Kind, type RateFields } from "./provider-onboarding";

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block min-w-0 space-y-1.5 text-sm font-medium">
      <span className="block">{label}</span>
      {children}
    </label>
  );
}

export function RateInputs({ kind, rates, change }: {
  kind: Kind;
  rates: RateFields;
  change: (rates: RateFields) => void;
}) {
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <Field label={kind === "text" ? "Input USD per 1M tokens" : `USD per ${rateUnit(kind)}`}>
        <Input required inputMode="decimal" placeholder="0.00" value={rates.amount}
          onChange={(event) => change({ ...rates, amount: event.target.value })} />
      </Field>
      {kind === "text" && (
        <Field label="Output USD per 1M tokens">
          <Input required inputMode="decimal" placeholder="0.00" value={rates.output}
            onChange={(event) => change({ ...rates, output: event.target.value })} />
        </Field>
      )}
    </div>
  );
}
