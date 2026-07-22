import { cn } from "@/lib/cn";

/**
 * Circular "x{n}" marker for batch requests (n > 1 results — images, videos, …), color
 * coded by batch size. The /15 backgrounds + 600/400 text pairs read correctly on both
 * the light and dark themes. Renders nothing for single-result requests.
 */
export function BatchBadge({ count, className }: { count: number; className?: string }) {
  if (count <= 1) return null;
  const tone =
    count >= 10
      ? "border-rose-500/40 bg-rose-500/15 text-rose-600 dark:text-rose-400"
      : count >= 7
        ? "border-amber-500/40 bg-amber-500/15 text-amber-600 dark:text-amber-400"
        : count >= 4
          ? "border-violet-500/40 bg-violet-500/15 text-violet-600 dark:text-violet-400"
          : "border-sky-500/40 bg-sky-500/15 text-sky-600 dark:text-sky-400";
  return (
    <span
      title={`Batch of ${count}`}
      className={cn(
        "inline-grid h-7 w-7 shrink-0 place-items-center rounded-full border text-[11px] font-semibold",
        tone,
        className,
      )}
    >
      x{count}
    </span>
  );
}
