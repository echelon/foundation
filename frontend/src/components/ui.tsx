import { type ButtonHTMLAttributes, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { Link } from "react-router-dom";
import { Moon, Sun } from "lucide-react";
import { cn } from "@/lib/cn";
import { useTheme } from "@/lib/theme";

/* ── Brand mark ── */
export function Logo({ className, withText = true }: { className?: string; withText?: boolean }) {
  return (
    <Link to="/" className={cn("flex items-center gap-2 font-display font-extrabold", className)}>
      <span className="grid h-7 w-7 place-items-center rounded-lg brand-gradient text-white shadow-glow">✦</span>
      {withText && <span className="text-lg tracking-tight">Foundation</span>}
    </Link>
  );
}

/* ── Theme toggle (per-surface) ── */
export function ThemeToggle({ className }: { className?: string }) {
  const { mode, toggle } = useTheme();
  return (
    <button
      onClick={toggle}
      aria-label="Toggle day / night"
      className={cn(
        "grid h-9 w-9 place-items-center rounded-lg border border-border bg-surface text-muted transition hover:text-fg hover:border-brand/50",
        className,
      )}
    >
      {mode === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
    </button>
  );
}

/* ── Button ── */
type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "outline";
  size?: "sm" | "md" | "lg";
};
export function Button({ variant = "primary", size = "md", className, ...props }: ButtonProps) {
  return (
    <button
      className={cn(
        "inline-flex items-center justify-center gap-2 rounded-lg font-semibold transition active:scale-[.98] disabled:cursor-not-allowed disabled:opacity-50",
        size === "sm" && "h-8 px-3 text-sm",
        size === "md" && "h-10 px-4 text-sm",
        size === "lg" && "h-12 px-6 text-base",
        variant === "primary" && "brand-gradient text-white shadow-glow hover:brightness-110",
        variant === "secondary" && "bg-surface2 text-fg hover:bg-border/60",
        variant === "outline" && "border border-border bg-surface text-fg hover:border-brand/60",
        variant === "ghost" && "text-muted hover:bg-surface2 hover:text-fg",
        className,
      )}
      {...props}
    />
  );
}

/* ── Card ── */
export function Card({ className, children }: { className?: string; children: ReactNode }) {
  return <div className={cn("rounded-2xl border border-border bg-surface", className)}>{children}</div>;
}

/* ── Badge ── */
export function Badge({ className, children, tone = "muted" }: { className?: string; children: ReactNode; tone?: "muted" | "brand" | "green" | "amber" | "red" }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium",
        tone === "muted" && "bg-surface2 text-muted",
        tone === "brand" && "bg-brand/12 text-brand",
        tone === "green" && "bg-emerald-500/12 text-emerald-500",
        tone === "amber" && "bg-amber-500/15 text-amber-500",
        tone === "red" && "bg-rose-500/15 text-rose-500",
        className,
      )}
    >
      {children}
    </span>
  );
}

/* ── Form controls ── */
export function Label({ children, hint }: { children: ReactNode; hint?: string }) {
  return (
    <label className="mb-1.5 flex items-center justify-between text-sm font-medium text-fg">
      <span>{children}</span>
      {hint && <span className="font-normal text-muted">{hint}</span>}
    </label>
  );
}

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={cn(
        "h-10 w-full rounded-lg border border-border bg-bg px-3 text-sm text-fg outline-none transition placeholder:text-muted focus:border-brand focus:ring-2 focus:ring-brand/25",
        props.className,
      )}
    />
  );
}

export function Textarea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      {...props}
      className={cn(
        "w-full rounded-lg border border-border bg-bg px-3 py-2.5 text-sm text-fg outline-none transition placeholder:text-muted focus:border-brand focus:ring-2 focus:ring-brand/25",
        props.className,
      )}
    />
  );
}

export function Select(props: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      {...props}
      className={cn(
        "h-10 w-full rounded-lg border border-border bg-bg px-3 text-sm text-fg outline-none transition focus:border-brand focus:ring-2 focus:ring-brand/25",
        props.className,
      )}
    />
  );
}

export function Slider({ value, min, max, step, onChange }: { value: number; min: number; max: number; step?: number; onChange: (v: number) => void }) {
  return (
    <input
      type="range"
      min={min}
      max={max}
      step={step ?? 1}
      value={value}
      onChange={(e) => onChange(Number(e.target.value))}
      className="h-2 w-full cursor-pointer appearance-none rounded-full bg-surface2 accent-[hsl(var(--brand))]"
    />
  );
}

/* ── Layout container ── */
export function Container({ className, children }: { className?: string; children: ReactNode }) {
  return <div className={cn("mx-auto w-full max-w-7xl px-5 sm:px-8", className)}>{children}</div>;
}
