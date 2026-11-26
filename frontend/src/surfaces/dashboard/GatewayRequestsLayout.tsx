import { Link, NavLink, Outlet, useMatch } from "react-router-dom";
import { ArrowUpRight } from "lucide-react";
import { cn } from "@/lib/cn";
import { withProject } from "@/lib/gateway";
import { useOrg } from "@/lib/org";
import { useProject } from "@/surfaces/projects/ProjectScope";

const linkClass = ({ isActive }: { isActive: boolean }) => cn(
  "border-b-2 px-3 py-2 text-sm font-semibold transition-colors",
  isActive ? "border-brand text-brand" : "border-transparent text-muted hover:text-fg",
);

export default function GatewayRequestsLayout() {
  const { withScope } = useOrg();
  const { project, scope } = useProject();
  const history = useMatch("/dashboard/:accountId?/requests");

  return (
    <div className="mx-auto max-w-7xl space-y-5">
      <h1 className="font-display text-4xl font-extrabold tracking-tight sm:text-5xl">Requests</h1>
      <div className="flex flex-wrap items-end justify-between gap-x-4 gap-y-1 border-b border-border">
        <nav aria-label="Requests navigation" className="flex flex-wrap gap-2">
          <NavLink to={withProject(withScope("/requests"), project?.id)} end className={linkClass}>History</NavLink>
          {project?.role === "admin" && (
            <NavLink to={withProject(withScope("/requests/exports"), project.id)} end className={linkClass}>Exports</NavLink>
          )}
        </nav>
        {history && <div className="flex flex-wrap gap-x-3 px-3 py-2 text-xs font-medium text-muted">
          <Link className="hover:text-brand hover:underline" to="generation">Generation jobs</Link>
          {scope && project?.role !== "billing_viewer" && (
            <Link className="inline-flex items-center gap-1 hover:text-brand hover:underline"
              to={withProject("/routing", scope.project_id, scope.account_id)}>
              Routing diagnostics <ArrowUpRight className="h-3 w-3" aria-hidden />
            </Link>
          )}
        </div>}
      </div>
      <Outlet />
    </div>
  );
}
