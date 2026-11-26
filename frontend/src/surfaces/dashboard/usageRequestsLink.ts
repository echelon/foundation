import type { ProjectScope } from "@/lib/gateway";
import type { UsageReport } from "@/surfaces/comparison/api";

export function usageRequestsLink(scope: ProjectScope, report: UsageReport, group?: { key: string | null }) {
  const params = new URLSearchParams({
    project_id: scope.project_id,
    from: report.window.from,
    to: report.window.to,
  });
  if (report.client_ref) params.set("client_ref", report.client_ref);
  if (report.app_ref) params.set("app_ref", report.app_ref);
  if (report.provider) params.set("provider", report.provider);
  if (report.model) params.set("model", report.model);

  if (group) {
    const { key } = group;
    switch (report.group_by) {
      case "client":
      case "app": {
        const reference = `${report.group_by}_ref`;
        if (key === null) {
          params.delete(reference);
          params.set(`${report.group_by}_unattributed`, "true");
        } else params.set(reference, key);
        break;
      }
      case "provider":
        if (key !== null) params.set("provider", key);
        break;
      case "model":
        if (key !== null) {
          params.set("model", key);
          params.set("exact_model", "true");
        }
        break;
      case "day":
        if (key !== null) {
          const tomorrow = new Date(`${key}T00:00:00Z`);
          tomorrow.setUTCDate(tomorrow.getUTCDate() + 1);
          params.set("from", key);
          params.set("to", tomorrow.toISOString().slice(0, 10));
        }
        break;
    }
  }
  return `/dashboard/${encodeURIComponent(scope.account_id)}/requests?${params}`;
}
