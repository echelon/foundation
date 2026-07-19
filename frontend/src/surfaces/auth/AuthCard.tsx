import { type ReactNode } from "react";
import { Card } from "@/components/ui";

/** Shared shell for the auth forms. */
export function AuthCard({ title, subtitle, children, footer }: { title: string; subtitle: string; children: ReactNode; footer: ReactNode }) {
  return (
    <div className="w-full max-w-sm">
      <Card className="p-7 shadow-glow">
        <h1 className="font-display text-2xl font-bold tracking-tight">{title}</h1>
        <p className="mt-1 text-sm text-muted">{subtitle}</p>
        <div className="mt-6 space-y-4">{children}</div>
      </Card>
      <p className="mt-5 text-center text-sm text-muted">{footer}</p>
    </div>
  );
}

export function SsoRow() {
  return (
    <div className="grid grid-cols-2 gap-3">
      {["Google", "GitHub"].map((p) => (
        <button
          key={p}
          className="flex h-10 items-center justify-center gap-2 rounded-lg border border-border bg-surface text-sm font-medium text-fg transition hover:border-brand/50"
        >
          {p}
        </button>
      ))}
    </div>
  );
}

export function Divider() {
  return (
    <div className="flex items-center gap-3 text-xs text-muted">
      <span className="h-px flex-1 bg-border" />
      or
      <span className="h-px flex-1 bg-border" />
    </div>
  );
}
