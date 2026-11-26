import { Link } from "react-router-dom";
import { GatewayOperations } from "./GatewaySetup";
import { useOrg } from "@/lib/org";
import { ProviderManagement } from "@/surfaces/projects/providers/ProviderManagement";
import { ProjectProviderKeys } from "@/surfaces/projects/ProjectProviderKeys";
import { ProjectDisclosure } from "@/surfaces/projects/ProjectDisclosure";
import { useProject } from "@/surfaces/projects/ProjectScope";

export default function Providers({ operations = false }: { operations?: boolean }) {
  const { current, withScope } = useOrg();
  const { projects, project } = useProject();
  if (!current) return null;

  const scope = { account_id: current.account_id };
  const canManage = current.role === "owner" || current.role === "admin";

  return (
    <div className="mx-auto max-w-7xl space-y-4">
      <header className="border-b border-border pb-4">
        <h1 className="font-display text-4xl font-extrabold tracking-tight sm:text-5xl">Providers</h1>
      </header>
      {canManage ? operations ? (
        <section className="space-y-4">
          <div className="space-y-1">
            <Link to={withScope("/providers")} className="text-sm font-semibold text-brand hover:underline">Back to providers</Link>
            <h2 className="text-xl font-semibold">Gateway operations</h2>
          </div>
          <GatewayOperations accountId={scope.account_id} projectId={project?.id} />
        </section>
      ) : (
        <ProviderManagement key={current.account_id} scope={scope} projects={projects} operations={withScope("/providers/operations")}>
          <ProjectDisclosure title="Provider credentials" description="Organization defaults" defer>
            <ProjectProviderKeys scope={scope} />
          </ProjectDisclosure>
        </ProviderManagement>
      ) : <p className="text-sm text-muted">Organization owners and admins can manage providers.</p>}
    </div>
  );
}
