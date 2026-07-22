import { cn } from "@/lib/cn";

/**
 * Circular "x{n}" marker for batch requests (n > 1 results — images, videos, …), color
 * coded by batch size. Solid 100/950 backgrounds (not alpha tints) so it stays readable
 * when overlapping thumbnails, with 700/300 text pairs for both themes. Renders nothing
 * for single-result requests.
 */
export function BatchBadge({ count, className }: { count: number; className?: string }) {
  if (count <= 1) return null;
  const tone =
    count >= 10
      ? "border-rose-500/40 bg-rose-100 text-rose-700 dark:bg-rose-950 dark:text-rose-300"
      : count >= 7
        ? "border-amber-500/40 bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-300"
        : count >= 4
          ? "border-violet-500/40 bg-violet-100 text-violet-700 dark:bg-violet-950 dark:text-violet-300"
          : "border-sky-500/40 bg-sky-100 text-sky-700 dark:bg-sky-950 dark:text-sky-300";
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
