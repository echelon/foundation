import { useMemo, useState } from "react";
import Prism from "prismjs";
import "prismjs/components/prism-bash";
import "prismjs/components/prism-json";
import "prismjs/components/prism-python";
import { Check, Copy } from "lucide-react";
import { cn } from "@/lib/cn";

export type CodeLang = "bash" | "javascript" | "python" | "json";

/** A single syntax-highlighted code block. Token colors are theme-aware (see index.css). */
export function CodeBlock({ code, lang, className }: { code: string; lang: CodeLang; className?: string }) {
  const html = useMemo(() => {
    const grammar = Prism.languages[lang] ?? Prism.languages.clike;
    return Prism.highlight(code, grammar, lang);
  }, [code, lang]);

  return (
    <pre className={cn("code-hl scrollbar-thin overflow-x-auto p-4 text-[13px] leading-relaxed", className)}>
      <code className={`language-${lang}`} dangerouslySetInnerHTML={{ __html: html }} />
    </pre>
  );
}

export type CodeTab = { label: string; lang: CodeLang; code: string };

/** Tabbed multi-language code sample with a copy button. */
export function CodeTabs({ tabs, className }: { tabs: CodeTab[]; className?: string }) {
  const [active, setActive] = useState(0);
  const [copied, setCopied] = useState(false);
  const tab = tabs[active];

  const copy = () => {
    navigator.clipboard?.writeText(tab.code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <div className={cn("overflow-hidden rounded-xl border border-border bg-surface", className)}>
      <div className="flex items-center justify-between border-b border-border bg-surface2/40 pr-2">
        <div className="flex">
          {tabs.map((t, i) => (
            <button
              key={t.label}
              onClick={() => setActive(i)}
              className={cn(
                "relative px-4 py-2.5 text-xs font-medium transition",
                i === active ? "text-fg" : "text-muted hover:text-fg",
              )}
            >
              {t.label}
              {i === active && <span className="absolute inset-x-2 bottom-0 h-0.5 rounded-full brand-gradient" />}
            </button>
          ))}
        </div>
        <button onClick={copy} className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted transition hover:text-fg">
          {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <CodeBlock code={tab.code} lang={tab.lang} />
    </div>
  );
}
