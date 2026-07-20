import { useCallback, useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { Laptop, Loader2, RefreshCw, ShieldCheck } from "lucide-react";
import { Badge, Button, Card, Input, Label } from "@/components/ui";
import { AvatarEditorModal } from "@/components/AvatarEditorModal";
import { UserAvatar } from "@/components/UserAvatar";
import { api, ApiError, type ApiSession } from "@/lib/api";
import { useAuth } from "@/lib/auth";

export default function Settings() {
  const { user } = useAuth();
  if (!user) return null;

  return (
    <div className="max-w-3xl space-y-8">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">Settings</h1>
        <p className="mt-1 text-muted">Manage your profile, security, and sessions.</p>
      </div>

      {/* ── Profile ── */}
      <Card className="p-6">
        <h2 className="font-display font-semibold">Profile</h2>
        <div className="mt-4 flex items-center gap-4">
          <UserAvatar user={user} className="h-16 w-16 text-xl" />
          <div>
            <div className="text-sm font-medium">{user.display_name ?? "—"}</div>
            <div className="flex items-center gap-2 text-sm text-muted">
              {user.email}
              {user.email_verified ? (
                <Badge tone="green">verified</Badge>
              ) : (
                <Badge tone="amber">unverified</Badge>
              )}
            </div>
          </div>
        </div>
        <AvatarManager />
      </Card>

      <ChangeEmailCard />
      <ChangePasswordCard />
      <SessionsCard />
    </div>
  );
}

/* ── Avatar management ── */

function AvatarManager() {
  const { user, refresh } = useAuth();
  const fileInput = useRef<HTMLInputElement>(null);
  const [editing, setEditing] = useState<File | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice>(null);

  const avatar = user?.avatar;

  // Run one avatar mutation with busy/notice handling, then refresh the user.
  const act = async (key: string, fn: () => Promise<unknown>, okText?: string) => {
    setBusy(key);
    setNotice(null);
    try {
      await fn();
      await refresh();
      if (okText) setNotice({ tone: "ok", text: okText });
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Something went wrong." });
    } finally {
      setBusy(null);
    }
  };

  const pickFile = (files: FileList | null) => {
    const file = files?.[0];
    if (file) setEditing(file);
    if (fileInput.current) fileInput.current.value = ""; // allow re-picking the same file
  };

  return (
    <div className="mt-5 border-t border-border pt-4">
      <input
        ref={fileInput}
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        className="hidden"
        onChange={(e) => pickFile(e.target.files)}
      />

      {/* Photo controls */}
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" onClick={() => fileInput.current?.click()} disabled={busy !== null}>
          {avatar?.has_custom ? "Replace photo" : "Upload photo"}
        </Button>
        {avatar?.has_custom && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => act("remove", api.removeAvatar, "Photo removed.")}
            disabled={busy !== null}
            className="border-rose-500/40 text-rose-500 hover:border-rose-500"
          >
            {busy === "remove" ? "…" : "Remove photo"}
          </Button>
        )}
      </div>

      {/* Gravatar controls */}
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <span className="text-xs text-muted">Gravatar:</span>
        {avatar?.has_gravatar && !avatar.is_gravatar_dismissed && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => act("dismiss", api.dismissGravatar, "Gravatar hidden. It's kept — re-enable any time.")}
            disabled={busy !== null}
          >
            {busy === "dismiss" ? "…" : "Hide"}
          </Button>
        )}
        {avatar?.is_gravatar_dismissed && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => act("restore", api.restoreGravatar, "Gravatar re-enabled.")}
            disabled={busy !== null}
          >
            {busy === "restore" ? "…" : "Re-enable"}
          </Button>
        )}
        <Button
          variant="outline"
          size="sm"
          onClick={() =>
            act("refresh", api.refreshGravatar, "Checking gravatar.com — this takes a few seconds; reload to see the result.")
          }
          disabled={busy !== null}
        >
          <RefreshCw className="h-3.5 w-3.5" />
          {busy === "refresh" ? "…" : "Refresh"}
        </Button>
        {avatar && !avatar.gravatar_checked && <Badge tone="amber">checking…</Badge>}
        {avatar?.has_gravatar && avatar.is_gravatar_dismissed && <Badge tone="muted">hidden</Badge>}
        {avatar?.source === "gravatar" && <Badge tone="green">showing</Badge>}
      </div>

      <NoticeRow notice={notice} />

      {editing && (
        <AvatarEditorModal file={editing} onClose={() => setEditing(null)} onSaved={() => refresh()} />
      )}
    </div>
  );
}

/* ── Change email ── */

function ChangeEmailCard() {
  const { user, refresh } = useAuth();
  const [newEmail, setNewEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setNotice(null);
    try {
      const r = await api.changeEmail(newEmail, password);
      await refresh(); // email + verified flag changed
      setNewEmail("");
      setPassword("");
      setNotice({ tone: "ok", text: `Email changed to ${r.email}. It's unverified until you confirm it.` });
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Something went wrong." });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="p-6">
      <h2 className="font-display font-semibold">Email address</h2>
      <p className="mt-1 text-sm text-muted">
        Currently <span className="font-medium text-fg">{user?.email}</span>. Changing it resets verification and is
        recorded in your account's audit log.
      </p>
      <form className="mt-4 grid gap-4 sm:grid-cols-2" onSubmit={submit}>
        <div>
          <Label>New email</Label>
          <Input
            type="email"
            placeholder="new@company.com"
            value={newEmail}
            onChange={(e) => setNewEmail(e.target.value)}
            required
          />
        </div>
        <div>
          <Label hint="required to confirm it's you">Current password</Label>
          <Input type="password" value={password} onChange={(e) => setPassword(e.target.value)} required />
        </div>
        <div className="sm:col-span-2">
          <NoticeRow notice={notice} />
          <Button type="submit" disabled={busy}>
            {busy ? "Saving…" : "Change email"}
          </Button>
        </div>
      </form>
    </Card>
  );
}

/* ── Change password ── */

function ChangePasswordCard() {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  const [logoutOthers, setLogoutOthers] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (next !== confirm) {
      setNotice({ tone: "err", text: "New passwords don't match." });
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      await api.changePassword(current, next, logoutOthers);
      setCurrent("");
      setNext("");
      setConfirm("");
      setNotice({
        tone: "ok",
        text: logoutOthers ? "Password changed. Other devices were signed out." : "Password changed.",
      });
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Something went wrong." });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="p-6">
      <div className="flex items-center gap-2">
        <ShieldCheck className="h-4 w-4 text-brand" />
        <h2 className="font-display font-semibold">Password</h2>
      </div>
      <form className="mt-4 grid gap-4 sm:grid-cols-3" onSubmit={submit}>
        <div>
          <Label>Current password</Label>
          <Input type="password" value={current} onChange={(e) => setCurrent(e.target.value)} required />
        </div>
        <div>
          <Label>New password</Label>
          <Input
            type="password"
            placeholder="At least 8 characters"
            value={next}
            onChange={(e) => setNext(e.target.value)}
            required
          />
        </div>
        <div>
          <Label>Confirm new</Label>
          <Input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} required />
        </div>
        <div className="sm:col-span-3">
          <label className="flex items-center gap-2 text-sm text-muted">
            <input
              type="checkbox"
              checked={logoutOthers}
              onChange={(e) => setLogoutOthers(e.target.checked)}
              className="h-4 w-4 accent-[hsl(var(--brand))]"
            />
            Also sign out my other devices
          </label>
          <NoticeRow notice={notice} />
          <Button type="submit" className="mt-3" disabled={busy}>
            {busy ? "Saving…" : "Change password"}
          </Button>
        </div>
      </form>
    </Card>
  );
}

/* ── Sessions ── */

function SessionsCard() {
  const [sessions, setSessions] = useState<ApiSession[] | null>(null);
  const [notice, setNotice] = useState<Notice>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const r = await api.listSessions();
      setSessions(r.sessions);
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Couldn't load sessions." });
      setSessions([]);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const terminate = async (id: string) => {
    setBusyId(id);
    setNotice(null);
    try {
      await api.revokeSession(id);
      await load();
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Couldn't terminate that session." });
    } finally {
      setBusyId(null);
    }
  };

  const terminateOthers = async () => {
    setBusyId("others");
    setNotice(null);
    try {
      await api.revokeOtherSessions();
      await load();
      setNotice({ tone: "ok", text: "All other sessions were signed out." });
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Couldn't sign out other sessions." });
    } finally {
      setBusyId(null);
    }
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between">
        <h2 className="font-display font-semibold">Sessions</h2>
        <Button variant="outline" size="sm" onClick={terminateOthers} disabled={busyId !== null}>
          {busyId === "others" ? "Signing out…" : "Sign out other sessions"}
        </Button>
      </div>
      <p className="mt-1 text-sm text-muted">Everywhere you're currently signed in.</p>
      <NoticeRow notice={notice} />

      {sessions === null ? (
        <div className="mt-4 flex items-center gap-2 text-sm text-muted">
          <Loader2 className="h-4 w-4 animate-spin" /> Loading sessions…
        </div>
      ) : sessions.length === 0 ? (
        <p className="mt-4 text-sm text-muted">No live sessions found.</p>
      ) : (
        <div className="mt-4 space-y-2">
          {sessions.map((s) => (
            <div
              key={s.id}
              className="flex items-center justify-between rounded-lg border border-border bg-surface2/40 px-4 py-3"
            >
              <div className="flex min-w-0 items-center gap-3">
                <Laptop className="h-4 w-4 shrink-0 text-muted" />
                <div className="min-w-0">
                  <div className="flex items-center gap-2 text-sm font-medium text-fg">
                    <span className="truncate">{describeAgent(s.user_agent)}</span>
                    {s.this_device && <Badge tone="brand">this device</Badge>}
                  </div>
                  <div className="truncate text-xs text-muted">
                    {s.ip ?? "unknown ip"} · signed in {shortDate(s.created_at)}
                  </div>
                </div>
              </div>
              {!s.this_device && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => terminate(s.id)}
                  disabled={busyId !== null}
                  className="shrink-0 border-rose-500/40 text-rose-500 hover:border-rose-500"
                >
                  {busyId === s.id ? "…" : "Terminate"}
                </Button>
              )}
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

/* ── Small helpers ── */

type Notice = { tone: "ok" | "err"; text: string } | null;

function NoticeRow({ notice }: { notice: Notice }): ReactNode {
  if (!notice) return null;
  return (
    <p
      className={
        notice.tone === "ok"
          ? "mt-3 rounded-lg bg-emerald-500/10 px-3 py-2 text-sm text-emerald-500"
          : "mt-3 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500"
      }
    >
      {notice.text}
    </p>
  );
}

function describeAgent(ua: string | null): string {
  if (!ua) return "Unknown device";
  if (ua.includes("iPhone")) return "iPhone";
  if (ua.includes("Android")) return "Android";
  if (ua.includes("Macintosh")) return uaBrowser(ua, "Mac");
  if (ua.includes("Windows")) return uaBrowser(ua, "Windows");
  if (ua.includes("Linux")) return uaBrowser(ua, "Linux");
  return ua.slice(0, 48);
}

function uaBrowser(ua: string, os: string): string {
  if (ua.includes("Edg/")) return `Edge on ${os}`;
  if (ua.includes("Firefox/")) return `Firefox on ${os}`;
  if (ua.includes("Chrome/")) return `Chrome on ${os}`;
  if (ua.includes("Safari/")) return `Safari on ${os}`;
  return os;
}

function shortDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" }) + " " +
    d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}
