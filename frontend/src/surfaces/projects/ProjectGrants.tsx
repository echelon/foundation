import { useEffect, useState, type FormEvent } from "react";
import { Button, Card, Input, Select } from "@/components/ui";
import { api, type ApiMember } from "@/lib/api";
import { errorMessage, type ProjectScope } from "@/lib/gateway";
import { grantRoles, projectsApi, type Grant, type GrantRole } from "./api";

const memberName = (member: ApiMember) => member.display_name ? `${member.display_name} · ${member.email}` : member.email;

export function ProjectGrants({ scope }: { scope: ProjectScope }) {
  const [data, setData] = useState<{ grants: Grant[]; members: ApiMember[] }>();
  const [revision, setRevision] = useState(0);
  const [error, setError] = useState("");
  const [memberId, setMemberId] = useState("");
  const [role, setRole] = useState<GrantRole>("developer");
  const [client, setClient] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([projectsApi.grants(scope, controller.signal), api.listMembers(scope.account_id)]).then(([grants, { members }]) => {
      if (!controller.signal.aborted) setData({ grants, members });
    }).catch((error) => {
      if (!controller.signal.aborted) setError(errorMessage(error));
    });
    return () => controller.abort();
  }, [scope, revision]);

  function edit(id: string) {
    const grant = data?.grants.find((item) => item.user_id === id);
    setMemberId(id);
    setRole(grant?.role ?? "developer");
    setClient(grant?.client_ref ?? "");
  }

  async function change(action: () => Promise<void>) {
    setSaving(true);
    setError("");
    try {
      await action();
      setData(undefined);
      edit("");
      setRevision((previous) => previous + 1);
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  }

  function save(event: FormEvent) {
    event.preventDefault();
    if (!memberId || saving || (role === "client_viewer" && !client.trim())) return;
    void change(() => projectsApi.grant(scope, { user_id: memberId, role, client_ref: role === "client_viewer" ? client.trim() : null }));
  }

  const members = data?.members.filter((member) => member.role === "member") ?? [];
  return (
    <Card className="min-w-0 space-y-4 p-4 sm:p-5">
      <div>
        <h3 className="font-display text-xl font-semibold">Member grants</h3>
        <p className="mt-1 text-sm text-muted">Owners and admins already have project access.</p>
      </div>
      {data && members.length > 0 && (
        <form onSubmit={save} className="flex flex-wrap items-end gap-3">
          <label className="block min-w-0 w-full max-w-xs space-y-1 text-sm font-medium">Organization member
            <Select required value={memberId} disabled={saving} onChange={(event) => edit(event.target.value)}>
              <option value="">Choose a member</option>
              {members.map((member) => <option key={member.user_id} value={member.user_id}>{memberName(member)}</option>)}
            </Select>
          </label>
          <label className="block min-w-0 w-full max-w-44 space-y-1 text-sm font-medium">Project role
            <Select value={role} disabled={saving} onChange={(event) => setRole(event.target.value as GrantRole)}>
              {Object.entries(grantRoles).map(([id, label]) => <option key={id} value={id}>{label}</option>)}
            </Select>
          </label>
          {role === "client_viewer" && <label className="block min-w-0 w-full max-w-52 space-y-1 text-sm font-medium">Client reference
            <Input value={client} required maxLength={64} disabled={saving} onChange={(event) => setClient(event.target.value)} />
          </label>}
          <Button type="submit" disabled={!memberId || saving}>Save access</Button>
        </form>
      )}
      {data && !members.length && <p className="text-sm text-muted">Invite an organization member from Team to grant them access here.</p>}
      {data && data.grants.length > 0 && (
        <ul className="divide-y divide-border border-t border-border">
          {data.grants.map((grant) => {
            const member = data.members.find((item) => item.user_id === grant.user_id);
            const label = member ? memberName(member) : grant.user_id;
            return (
              <li key={grant.user_id} className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm">
                <div className="min-w-0 break-all">
                  <p className="font-medium">{label}</p>
                  <p className="text-muted">{grantRoles[grant.role]}{grant.client_ref && ` · ${grant.client_ref}`}</p>
                </div>
                <Button variant="outline" size="sm" disabled={saving} onClick={() => {
                  if (window.confirm(`Remove project access for ${label}?`)) void change(() => projectsApi.revokeGrant(scope, grant.user_id));
                }}>Remove access</Button>
              </li>
            );
          })}
        </ul>
      )}
      {!data && !error && <p role="status" className="text-sm text-muted">Loading member grants…</p>}
      {error && <p role="alert" className="text-sm text-rose-600 dark:text-rose-300">{error} {!data && <button type="button" className="underline" onClick={() => { setError(""); setRevision((value) => value + 1); }}>Retry</button>}</p>}
    </Card>
  );
}
