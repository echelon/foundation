import { Link } from "react-router-dom";
import { Badge } from "@/components/ui";
import { MODEL_BY_SLUG } from "@/lib/catalog";
import { type JobStatus, type Request } from "@/lib/mock-api";

export function StatusPill({ status }: { status: JobStatus }) {
  if (status === "succeeded") return <Badge tone="green">succeeded</Badge>;
  if (status === "failed") return <Badge tone="red">failed</Badge>;
  return <Badge tone="amber">running</Badge>;
}

/** Compact relative-ish time from an ISO string (deterministic — just formats). */
function shortTime(iso: string): string {
  // iso like 2026-07-19T14:22:10Z → "Jul 19, 14:22"
  const [date, time] = iso.replace("Z", "").split("T");
  const [, m, d] = date.split("-");
  const months = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  return `${months[Number(m)]} ${Number(d)}, ${time.slice(0, 5)}`;
}

export function RequestRow({ r }: { r: Request }) {
  const model = MODEL_BY_SLUG[r.modelSlug];
  return (
    <tr className="border-b border-border/60 last:border-0 hover:bg-surface2/40">
      <td className="px-5 py-3">
        <div className="font-mono text-[13px] text-fg">{r.id}</div>
        <div className="text-xs text-muted">{shortTime(r.createdAt)}</div>
      </td>
      <td className="px-5 py-3">
        <Link to={`/models/${r.modelSlug}`} className="font-medium text-fg hover:text-brand">
          {model?.name ?? r.modelSlug}
        </Link>
        <div className="truncate text-xs text-muted" style={{ maxWidth: "16rem" }}>
          {r.summary}
        </div>
      </td>
      <td className="px-5 py-3">
        <Badge>{r.source}</Badge>
      </td>
      <td className="px-5 py-3">
        {r.endUser ? (
          <span className="font-mono text-xs text-muted">{r.endUser}</span>
        ) : r.mine ? (
          <span className="text-xs text-brand">you</span>
        ) : (
          <span className="text-xs text-muted">—</span>
        )}
      </td>
      <td className="px-5 py-3">
        <StatusPill status={r.status} />
      </td>
      <td className="px-5 py-3 text-right font-medium text-fg">${r.costUsd.toFixed(r.costUsd < 0.1 ? 4 : 2)}</td>
    </tr>
  );
}
