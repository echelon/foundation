import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Copy, MailX, UserMinus } from "lucide-react";
import { Badge, Button, Card, Input, Label, Select } from "@/components/ui";
import { UserAvatar } from "@/components/UserAvatar";
import { api, ApiError, type ApiAccountUser, type ApiMember, type ApiPendingInvitation } from "@/lib/api";
import { useOrg } from "@/lib/org";

/**
 * The organization page: the member roster with roles, plus invite/remove for owners and
 * admins. Scoped to the org selected in the header switcher.
 */
export default function Organization() {
  const { current, loading } = useOrg();
  const [members, setMembers] = useState<ApiMember[] | null>(null);
  const [myRole, setMyRole] = useState<string>("member");
  const [notice, setNotice] = useState<Notice>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const [invites, setInvites] = useState<ApiPendingInvitation[] | null>(null);
  const [inviters, setInviters] = useState<Record<string, ApiAccountUser>>({});

  const load = useCallback(async () => {
    if (!current) return;
    try {
      const r = await api.listMembers(current.account_id);
      setMembers(r.members);
      setMyRole(r.your_role);
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Couldn't load members." });
      setMembers([]);
    }
  }, [current]);

  // Owner/admin only — a member's 403 just means no card (the role check below hides it too).
  const loadInvites = useCallback(async () => {
    if (!current) return;
    try {
      const r = await api.listInvitations(current.account_id);
      setInvites(r.invitations);
      const ids = [...new Set(r.invitations.map((i) => i.invited_by_user_id))];
      if (ids.length > 0) {
        api
          .accountUsers(current.account_id, ids)
          .then((res) => {
            const map: Record<string, ApiAccountUser> = {};
            for (const u of res.users) map[u.user_id] = u;
            setInviters(map);
          })
          .catch(() => {});
      }
    } catch {
      setInvites([]);
    }
  }, [current]);

  useEffect(() => {
    setMembers(null);
    setInvites(null);
    void load();
    void loadInvites();
  }, [load, loadInvites]);

  if (loading) return null;
  if (!current) return <p className="text-muted">You don't belong to any organization yet.</p>;

  const canManage = myRole === "owner" || myRole === "admin";

  const remove = async (member: ApiMember) => {
    setBusyId(member.user_id);
    setNotice(null);
    try {
      await api.removeMember(current.account_id, member.user_id);
      setNotice({ tone: "ok", text: `${member.display_name ?? member.email} was removed.` });
      await load();
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Couldn't remove that member." });
    } finally {
      setBusyId(null);
    }
  };

  // Removal visibility mirrors the server rules: never the owner, never yourself; admins can
  // only remove plain members.
  const canRemove = (m: ApiMember) =>
    canManage && !m.is_you && m.role !== "owner" && (myRole === "owner" || m.role === "member");

  return (
    <div className="max-w-3xl space-y-8">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">{current.name ?? "Organization"}</h1>
        <p className="mt-1 text-muted">
          Members and roles · you are <Badge tone="brand">{myRole}</Badge>
        </p>
      </div>

      <Card className="overflow-hidden">
        <div className="border-b border-border px-5 py-4">
          <h2 className="font-display font-semibold">Members</h2>
        </div>
        {members === null ? (
          <p className="px-5 py-4 text-sm text-muted">Loading…</p>
        ) : (
          <table className="w-full text-sm">
            <tbody>
              {members.map((m) => (
                <tr key={m.user_id} className="border-b border-border/60 last:border-0">
                  <td className="py-3 pl-5 pr-3">
                    <div className="flex items-center gap-3">
                      {/* Their chosen avatar: upload, gravatar, or initials. */}
                      <UserAvatar user={m} className="h-9 w-9 shrink-0 text-xs" />
                      <div className="min-w-0">
                        <div className="font-medium text-fg">
                          {m.display_name ?? "—"}
                          {m.is_you && <span className="ml-2 text-xs text-brand">you</span>}
                        </div>
                        <div className="truncate text-xs text-muted">{m.email}</div>
                      </div>
                    </div>
                  </td>
                  <td className="px-3 py-3">
                    <Badge tone={m.role === "owner" ? "brand" : m.role === "admin" ? "green" : "muted"}>{m.role}</Badge>
                  </td>
                  <td className="hidden px-3 py-3 text-xs text-muted sm:table-cell">
                    joined {new Date(m.joined_at).toLocaleDateString()}
                  </td>
                  <td className="py-3 pl-3 pr-5 text-right">
                    {canRemove(m) && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => remove(m)}
                        disabled={busyId !== null}
                        className="border-rose-500/40 text-rose-500 hover:border-rose-500"
                      >
                        <UserMinus className="h-3.5 w-3.5" />
                        {busyId === m.user_id ? "…" : "Remove"}
                      </Button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <NoticeRow notice={notice} className="px-5 pb-4" />
      </Card>

      {canManage && invites !== null && invites.length > 0 && (
        <PendingInvitesCard
          accountId={current.account_id}
          invites={invites}
          inviters={inviters}
          onChanged={loadInvites}
        />
      )}

      {canManage && (
        <InviteCard
          accountId={current.account_id}
          onInvited={async () => {
            await load();
            await loadInvites();
          }}
        />
      )}
    </div>
  );
}

/* ── Invites ── */

function InviteCard({ accountId, onInvited }: { accountId: string; onInvited: () => Promise<void> }) {
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<"admin" | "member">("member");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice>(null);
  const [invite, setInvite] = useState<{ code: string; join_url: string } | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setNotice(null);
    setInvite(null);
    try {
      const r = await api.inviteMember(accountId, email, role);
      setInvite({ code: r.code, join_url: r.join_url });
      setEmail("");
      await onInvited();
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Couldn't create the invitation." });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="p-6">
      <h2 className="font-display font-semibold">Invite a member</h2>
      <form className="mt-4 flex flex-wrap items-end gap-3" onSubmit={submit}>
        <div className="min-w-56 flex-1">
          <Label>Email</Label>
          <Input
            type="email"
            placeholder="teammate@company.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
        </div>
        <div>
          <Label>Role</Label>
          <Select value={role} onChange={(e) => setRole(e.target.value as "admin" | "member")}>
            <option value="member">Member</option>
            <option value="admin">Admin</option>
          </Select>
        </div>
        <Button type="submit" disabled={busy}>
          {busy ? "Inviting…" : "Invite"}
        </Button>
      </form>
      {invite && (
        <div className="mt-4 rounded-lg border border-border bg-surface2/40 px-4 py-3 text-sm">
          <p className="text-muted">
            Share this with them — code <code className="rounded bg-surface2 px-1.5 py-0.5 font-mono">{invite.code}</code>{" "}
            or the join link:
          </p>
          <div className="mt-2 flex items-center gap-2">
            <code className="min-w-0 flex-1 truncate rounded bg-surface2 px-2 py-1 font-mono text-xs">
              {window.location.origin}
              {invite.join_url}
            </code>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void navigator.clipboard.writeText(`${window.location.origin}${invite.join_url}`)}
            >
              <Copy className="h-3.5 w-3.5" />
              Copy
            </Button>
          </div>
        </div>
      )}
      <NoticeRow notice={notice} />
    </Card>
  );
}

/* ── Pending invitations ── */

function PendingInvitesCard({
  accountId,
  invites,
  inviters,
  onChanged,
}: {
  accountId: string;
  invites: ApiPendingInvitation[];
  inviters: Record<string, ApiAccountUser>;
  onChanged: () => Promise<void>;
}) {
  const [busyId, setBusyId] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice>(null);

  const cancel = async (inv: ApiPendingInvitation) => {
    if (!window.confirm(`Cancel the invitation to ${inv.email}? Its link and code stop working immediately.`)) return;
    setBusyId(inv.invitation_id);
    setNotice(null);
    try {
      await api.cancelInvitation(accountId, inv.invitation_id);
      setNotice({ tone: "ok", text: `Invitation to ${inv.email} cancelled.` });
      await onChanged();
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Couldn't cancel that invitation." });
    } finally {
      setBusyId(null);
    }
  };

  return (
    <Card className="overflow-hidden">
      <div className="border-b border-border px-5 py-4">
        <h2 className="font-display font-semibold">Pending invitations</h2>
        <p className="text-xs text-muted">Sent but not yet accepted. Cancelling kills the link and code.</p>
      </div>
      <table className="w-full text-sm">
        <tbody>
          {invites.map((inv) => {
            const inviter = inviters[inv.invited_by_user_id];
            return (
              <tr key={inv.invitation_id} className="border-b border-border/60 last:border-0">
                <td className="py-3 pl-5 pr-3">
                  <div className="font-medium text-fg">{inv.email}</div>
                  <div className="mt-0.5 flex flex-wrap items-center gap-1.5 text-xs text-muted">
                    <span>
                      invited {new Date(inv.created_at).toLocaleDateString()} by{" "}
                    </span>
                    {inviter ? (
                      <span className="inline-flex items-center gap-1">
                        <UserAvatar user={inviter} className="h-4 w-4 text-[8px]" />
                        {inviter.display_name ?? inviter.email}
                      </span>
                    ) : (
                      <span>a former member</span>
                    )}
                  </div>
                </td>
                <td className="px-3 py-3">
                  <Badge tone={inv.role === "admin" ? "green" : "muted"}>{inv.role}</Badge>
                </td>
                <td className="hidden px-3 py-3 text-xs text-muted sm:table-cell">
                  code <code className="rounded bg-surface2 px-1.5 py-0.5 font-mono">{inv.code}</code>
                </td>
                <td className="hidden px-3 py-3 text-xs text-muted sm:table-cell">
                  expires {new Date(inv.expires_at).toLocaleDateString()}
                </td>
                <td className="py-3 pl-3 pr-5 text-right">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => void cancel(inv)}
                    disabled={busyId !== null}
                    className="border-rose-500/40 text-rose-500 hover:border-rose-500"
                  >
                    <MailX className="h-3.5 w-3.5" />
                    {busyId === inv.invitation_id ? "…" : "Cancel"}
                  </Button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <NoticeRow notice={notice} className="px-5 pb-4" />
    </Card>
  );
}

/* ── shared notice ── */

type Notice = { tone: "ok" | "err"; text: string } | null;

function NoticeRow({ notice, className }: { notice: Notice; className?: string }) {
  if (!notice) return null;
  return (
    <p
      className={
        (notice.tone === "ok"
          ? "mt-3 rounded-lg bg-emerald-500/10 px-3 py-2 text-sm text-emerald-500 "
          : "mt-3 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500 ") + (className ?? "")
      }
    >
      {notice.text}
    </p>
  );
}
