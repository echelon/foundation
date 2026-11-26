import { type ComponentPropsWithRef, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { Link } from "react-router-dom";
import { ChevronDown, Moon, Sun } from "lucide-react";
import { cn } from "@/lib/cn";
import { useTheme } from "@/lib/theme";

/* ── Brand mark ── */
export function Logo({ className, withText = true }: { className?: string; withText?: boolean }) {
  return (
    <Link to="/" aria-label="TokenCraft home" className={cn("flex items-center gap-2 font-display font-extrabold", className)}>
      <img src="/brand/tokencraft-mark.png" alt="" width={32} height={32} className="h-8 w-8 shrink-0" />
      {withText && <span className="text-lg tracking-tight">TokenCraft</span>}
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
        "grid h-9 w-9 place-items-center border border-border bg-surface text-muted transition hover:text-fg hover:border-brand/50",
        className,
      )}
    >
      {mode === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
    </button>
  );
}

/* ── Button ── */
type ButtonProps = ComponentPropsWithRef<"button"> & {
  variant?: "primary" | "secondary" | "ghost" | "outline";
  size?: "sm" | "md" | "lg";
};
export function Button({ variant = "primary", size = "md", className, ...props }: ButtonProps) {
  return (
    <button
      className={cn(
        "inline-flex items-center justify-center gap-2 font-semibold transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand disabled:cursor-not-allowed disabled:opacity-50",
        size === "sm" && "h-8 px-3 text-sm",
        size === "md" && "h-10 px-4 text-sm",
        size === "lg" && "h-12 px-6 text-base",
        variant === "primary" && "brand-gradient hover:opacity-85",
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
        "inline-flex items-center gap-1 rounded-sm px-2 py-0.5 text-sm font-medium",
        tone === "muted" && "bg-surface2 text-muted",
        tone === "brand" && "bg-brand/12 text-brand",
        tone === "green" && "bg-emerald-500/10 text-emerald-800 dark:text-emerald-300",
        tone === "amber" && "bg-amber-500/15 text-amber-800 dark:text-amber-300",
        tone === "red" && "bg-rose-500/15 text-rose-800 dark:text-rose-300",
        className,
      )}
    >
      {children}
    </span>
  );
}

/* ── Form controls ── */
export function Label({ children, hint, htmlFor }: { children: ReactNode; hint?: string; htmlFor?: string }) {
  return (
    <label htmlFor={htmlFor} className="mb-1.5 flex items-center justify-between text-sm font-medium text-fg">
      <span>{children}</span>
      {hint && <span className="font-normal text-muted">{hint}</span>}
    </label>
  );
}

export function Checkbox({ className, ...props }: Omit<ComponentPropsWithRef<"input">, "type">) {
  return <input {...props} type="checkbox" className={cn(
    "h-4 w-4 shrink-0 cursor-pointer accent-brand focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand disabled:cursor-not-allowed disabled:opacity-55",
    className,
  )} />;
}

export function Input(props: ComponentPropsWithRef<"input">) {
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
    <span className="relative block">
      <select
        {...props}
        className={cn(
          "h-10 w-full appearance-none rounded-sm border border-border bg-bg pl-3 pr-8 text-sm text-fg outline-none transition focus:border-brand focus:ring-2 focus:ring-brand/25",
          props.className,
        )}
      />
      <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted" aria-hidden />
    </span>
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

/* ── Modal ── */
/** Overlay dialog (same pattern as the gallery's ExampleModal): click-outside or ✕ to close. */
export function Modal({
  open,
  onClose,
  children,
  className,
}: {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  className?: string;
}) {
  if (!open) return null;
  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className={cn("w-full max-w-sm rounded-2xl border border-border bg-surface p-6 shadow-glow", className)}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
