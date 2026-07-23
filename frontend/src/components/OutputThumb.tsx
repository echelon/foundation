/**
 * A tiny output thumbnail that handles BOTH media types: <img> for images and a
 * muted, non-playing <video> frame for videos (the browser shows the first frame).
 * Used by request-history rows in the playgrounds, dashboard, and admin console.
 */

import type { ApiJobOutput } from "@/lib/api";

export function OutputThumb({ output, className }: { output: ApiJobOutput; className?: string }) {
  const cls = className ?? "h-10 w-10 rounded-lg border border-border object-cover";
  if (output.mime.startsWith("video/")) {
    return <video src={output.url} muted playsInline preload="metadata" className={cls} />;
  }
  return <img src={output.url} alt="" className={cls} />;
}
