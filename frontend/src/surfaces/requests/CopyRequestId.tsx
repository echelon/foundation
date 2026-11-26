import { useEffect, useRef, useState } from "react";
import { Check, Copy } from "lucide-react";

export function CopyRequestId({ id }: { id: string }) {
  const [state, setState] = useState<"ready" | "copied" | "manual">("ready");
  const timer = useRef<number | undefined>(undefined);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => () => window.clearTimeout(timer.current), []);
  useEffect(() => {
    if (state === "manual") {
      input.current?.focus();
      input.current?.select();
    }
  }, [state]);

  async function copy() {
    window.clearTimeout(timer.current);
    try {
      await navigator.clipboard.writeText(id);
      setState("copied");
      timer.current = window.setTimeout(() => setState("ready"), 1800);
    } catch {
      setState("manual");
      input.current?.focus();
      input.current?.select();
    }
  }

  const copied = state === "copied";
  const short = id.startsWith("req_") && id.length > 12 ? "req_…" + id.slice(-8) : id;
  return (
    <div className="min-w-0">
      <button type="button" onClick={() => void copy()} aria-label="Copy request ID"
        title={copied ? "Copied" : "Copy " + id}
        className="inline-flex max-w-full items-center gap-1.5 rounded-md px-1.5 py-1.5 text-xs text-muted hover:bg-surface2 hover:text-fg focus-visible:outline focus-visible:outline-2 focus-visible:outline-brand">
        <code className="truncate">{short}</code>
        {copied ? <Check className="h-3.5 w-3.5 shrink-0" aria-hidden />
          : <Copy className="h-3.5 w-3.5 shrink-0" aria-hidden />}
      </button>
      <span role="status" className="sr-only">{copied ? "Full request ID copied." : ""}</span>
      {state === "manual" && <input ref={input} readOnly value={id} aria-label="Full request ID"
        onFocus={(event) => event.currentTarget.select()} onBlur={() => setState("ready")}
        onKeyDown={(event) => { if (event.key === "Escape") event.currentTarget.blur(); }}
        className="block max-w-full rounded border border-border bg-surface px-1.5 py-1 font-mono text-xs" />}
    </div>
  );
}
