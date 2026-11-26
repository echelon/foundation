import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { ArrowUpRight, ChevronDown, Info } from "lucide-react";

export const routingDocsPath = "/docs#project-routing-policy";

export function ProjectDisclosure({ title, description, help, defer, unmountOnClose, children }: {
  title: string;
  description?: string;
  help?: ReactNode;
  defer?: boolean;
  unmountOnClose?: boolean;
  children: ReactNode;
}) {
  const [visited, setVisited] = useState(false);
  return (
    <div className="relative rounded-xl border border-border">
      <details className="group/disclosure" onToggle={(event) => {
        const open = event.currentTarget.open;
        if (unmountOnClose) setVisited(open);
        else if (defer && open) setVisited(true);
      }}>
        <summary className={`flex cursor-pointer list-none items-center gap-3 rounded-xl py-3 pl-3.5 text-sm hover:bg-surface2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-brand sm:pl-4 [&::-webkit-details-marker]:hidden ${help ? "pr-12 sm:pr-14" : "pr-3.5 sm:pr-4"}`}>
          <ChevronDown className="h-4 w-4 shrink-0 -rotate-90 text-muted transition-transform group-open/disclosure:rotate-0 motion-reduce:transition-none" aria-hidden />
          <span className="min-w-0">
            <span className="block font-semibold">{title}</span>
            {description && <span className="mt-0.5 block text-xs leading-relaxed text-muted">{description}</span>}
          </span>
        </summary>
        <div className="space-y-4 border-t border-border p-3.5 sm:p-4">
          {((!defer && !unmountOnClose) || visited) && children}
        </div>
      </details>
      {help && <div className="absolute right-2 top-2 sm:right-3">{help}</div>}
    </div>
  );
}

export function PolicyInfo({ title, topic, children }: { title: string; topic: string; children: ReactNode }) {
  const id = useId();
  const container = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    function dismiss(event: PointerEvent) {
      if (event.target instanceof Node && !container.current?.contains(event.target)) setOpen(false);
    }
    document.addEventListener("pointerdown", dismiss, true);
    return () => document.removeEventListener("pointerdown", dismiss, true);
  }, [open]);

  return (
    <div ref={container} className="relative shrink-0" onKeyDown={(event) => {
      if (event.key === "Escape" && open) {
        event.stopPropagation();
        setOpen(false);
        button.current?.focus();
      }
    }} onBlur={(event) => {
      if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
    }}>
      <button ref={button} type="button" aria-label={`About ${title}`} aria-expanded={open} aria-controls={open ? id : undefined}
        onClick={() => setOpen(!open)}
        className="grid h-8 w-8 place-items-center rounded-md text-muted hover:bg-surface2 hover:text-fg focus-visible:outline focus-visible:outline-2 focus-visible:outline-brand aria-expanded:bg-surface2 aria-expanded:text-fg">
        <Info className="h-4 w-4" aria-hidden />
      </button>
      {open && <div id={id} className="absolute right-0 top-full z-30 mt-1 w-56 max-w-[calc(100vw-3rem)] space-y-2 rounded-lg border border-border bg-surface p-3 text-xs leading-relaxed text-fg shadow-lg">
        <p>{children}</p>
        <a href={`/docs#${topic}`} target="_blank" rel="noopener noreferrer"
          className="inline-flex items-center gap-1 font-semibold text-brand underline-offset-2 hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-brand">
          Read documentation <ArrowUpRight className="h-3 w-3" aria-hidden /><span className="sr-only"> (opens in a new tab)</span>
        </a>
      </div>}
    </div>
  );
}
