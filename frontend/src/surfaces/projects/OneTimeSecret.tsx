import { useState } from "react";
import { Button, Input } from "@/components/ui";

export function OneTimeSecret({ label, noun, value, metadata, dismiss }: {
  label: string; noun: string; value: string; metadata?: string; dismiss: () => void;
}) {
  const [copied, setCopied] = useState("");
  return (
    <div className="space-y-2 border border-brand/45 bg-brand/10 p-3">
      <p role="status" className="text-sm font-medium">Copy this {noun} now. It is shown only at issuance.</p>
      <Input aria-label={label} className="font-mono" value={value} readOnly spellCheck={false} autoComplete="off" />
      {metadata && <p className="text-xs text-muted">{metadata}</p>}
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" onClick={async () => {
          try {
            await navigator.clipboard.writeText(value);
            setCopied("Copied.");
          } catch {
            setCopied(`Select the ${noun} above to copy it.`);
          }
        }}>Copy {noun}</Button>
        <Button variant="ghost" size="sm" onClick={dismiss}>Dismiss {noun}</Button>
        <span role="status" className="text-sm text-muted">{copied}</span>
      </div>
    </div>
  );
}
