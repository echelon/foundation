import { useCallback, useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { Check, Image as ImageIcon, Laptop, Loader2, Palette, RefreshCw, ShieldCheck } from "lucide-react";
import { Badge, Button, Card, Input, Label } from "@/components/ui";
import { AvatarEditorModal } from "@/components/AvatarEditorModal";
import { UserAvatar, userInitials } from "@/components/UserAvatar";
import { api, ApiError, type ApiSession } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { MONOGRAM_GRADIENTS, monogramGradientCss } from "@/lib/gradients";

export default function Settings() {
  const { user, ssoProviders } = useAuth();
  if (!user) return null;

  const factoids: { label: string; on: boolean }[] = [
    { label: "password", on: user.has_password },
    { label: "Google SSO", on: ssoProviders.includes("google") },
    { label: "GitHub SSO", on: ssoProviders.includes("github") },
  ];

  return (
    <div className="max-w-3xl space-y-8">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">User settings</h1>
        <p className="mt-1 text-muted">Manage your profile, security, and sessions.</p>
      </div>

      {/* ── Profile — a 100% read-only snapshot; everything here is edited in the
             sections below. ── */}
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
        {/* Login-method factoids. */}
        <div className="mt-4 flex flex-wrap gap-2">
          {factoids.map((f) => (
            <Badge key={f.label} tone={f.on ? "green" : "muted"}>
              {f.on ? `has ${f.label}` : `no ${f.label}`}
            </Badge>
          ))}
        </div>
      </Card>

      {!user.email_verified && <VerifyEmailRow />}

      {/* ── Name ── */}
      <Card className="p-6">
        <h2 className="font-display font-semibold">Name</h2>
        <NameEditor />
      </Card>

      <AvatarCard />

      <ChangeEmailCard />
      <ChangePasswordCard />
      <SessionsCard />
    </div>
  );
}

/* ── Email verification ── */

function VerifyEmailRow() {
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice>(null);

  const send = async () => {
    setBusy(true);
    setNotice(null);
    try {
      const r = await api.requestEmailVerification();
      setNotice({ tone: "ok", text: r.message });
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Something went wrong." });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mt-4 rounded-lg border border-amber-500/30 bg-amber-500/5 px-4 py-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-sm text-muted">
          Your email isn't verified yet — we'll send a one-click confirmation link.
        </p>
        <Button variant="outline" size="sm" onClick={send} disabled={busy}>
          {busy ? "Sending…" : "Send verification email"}
        </Button>
      </div>
      <NoticeRow notice={notice} />
    </div>
  );
}

/* ── Display name ── */

function NameEditor() {
  const { user, refresh } = useAuth();
  const [name, setName] = useState(user?.display_name ?? "");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice>(null);

  const save = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setNotice(null);
    try {
      const trimmed = name.trim();
      await api.updateProfile(trimmed === "" ? null : trimmed);
      await refresh();
      setNotice({ tone: "ok", text: "Name updated. This change is visible in your account's audit log." });
    } catch (err) {
      setNotice({ tone: "err", text: err instanceof ApiError ? err.message : "Something went wrong." });
    } finally {
      setBusy(false);
    }
  };

  return (
    <form className="mt-3" onSubmit={save}>
      <div className="flex items-end gap-2">
        <div className="flex-1">
          <Label hint="shown to your team; changes are audit-logged">Name</Label>
          <Input placeholder="Ada Rivera" value={name} onChange={(e) => setName(e.target.value)} maxLength={100} />
        </div>
        <Button type="submit" size="sm" disabled={busy || (name.trim() || null) === (user?.display_name ?? null)}>
          {busy ? "Saving…" : "Save name"}
        </Button>
      </div>
      <NoticeRow notice={notice} />
    </form>
  );
}

/* ── Avatar ── */

function AvatarCard() {
  const { user, refresh } = useAuth();
  const fileInput = useRef<HTMLInputElement>(null);
  const [editing, setEditing] = useState<File | null>(null);
  const [pickingGradient, setPickingGradient] = useState(false);
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

  const thumb = (links: { cdn_url: string; thumbnail_template: string } | null | undefined) =>
    links ? links.thumbnail_template.replace("{WIDTH}", "320") : null;

  const selected = avatar?.preferred_source;

  return (
    <Card className="p-6">
      <h2 className="font-display font-semibold">Profile picture</h2>
      <p className="mt-1 text-sm text-muted">
        Pick what displays: your photo, your gravatar, or your initials.
        {avatar?.preferred_source === "not_yet_set" && " (Auto-picked until you choose.)"}
      </p>

      <input
        ref={fileInput}
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        className="hidden"
        onChange={(e) => pickFile(e.target.files)}
      />

      <div className="mt-4 grid grid-cols-3 gap-3 sm:gap-4">
        {/* ── Photo ── */}
        <AvatarOption
          label="Photo"
          selected={selected === "custom"}
          selectable={!!avatar?.has_custom}
          unavailableHint="upload a photo first"
          onSelect={() => act("prefer-custom", () => api.setAvatarPreference("custom"))}
          preview={
            thumb(avatar?.custom_links) ? (
              <img src={thumb(avatar?.custom_links)!} alt="" className="h-full w-full object-cover" />
            ) : (
              <EmptyPreview icon={<ImageIcon className="h-8 w-8" />} />
            )
          }
          action={
            avatar?.has_custom ? (
              <Button
                variant="outline"
                size="sm"
                className="w-full border-rose-500/40 text-rose-500 hover:border-rose-500"
                disabled={busy !== null}
                onClick={() => act("clear-photo", api.removeAvatar, "Photo removed.")}
              >
                {busy === "clear-photo" ? "…" : "Clear"}
              </Button>
            ) : (
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                disabled={busy !== null}
                onClick={() => fileInput.current?.click()}
              >
                Upload
              </Button>
            )
          }
        />

        {/* ── Gravatar ── */}
        <AvatarOption
          label="Gravatar"
          selected={selected === "gravatar"}
          selectable
          badge={avatar && !avatar.gravatar_checked ? "checking…" : undefined}
          onSelect={() => act("prefer-gravatar", () => api.setAvatarPreference("gravatar"))}
          preview={
            thumb(avatar?.gravatar_links) ? (
              <img src={thumb(avatar?.gravatar_links)!} alt="" className="h-full w-full object-cover" />
            ) : (
              <EmptyPreview icon={<ImageIcon className="h-8 w-8" />} />
            )
          }
          action={
            avatar?.has_gravatar ? (
              <Button
                variant="outline"
                size="sm"
                className="w-full border-rose-500/40 text-rose-500 hover:border-rose-500"
                disabled={busy !== null}
                onClick={() => act("remove-gravatar", api.removeGravatar, "Gravatar removed. Check brings it back.")}
              >
                {busy === "remove-gravatar" ? "…" : "Remove"}
              </Button>
            ) : (
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                disabled={busy !== null}
                onClick={() =>
                  act("check-gravatar", api.refreshGravatar, "Checking gravatar.com — takes a few seconds.")
                }
              >
                <RefreshCw className="h-3.5 w-3.5" />
                {busy === "check-gravatar" ? "…" : "Check"}
              </Button>
            )
          }
        />

        {/* ── Initials ── */}
        <AvatarOption
          label="Initials"
          selected={selected === "letter"}
          selectable
          onSelect={() => act("prefer-letter", () => api.setAvatarPreference("letter"))}
          preview={
            <div
              className="grid h-full w-full place-items-center font-display text-4xl font-bold text-white"
              style={{ background: monogramGradientCss(avatar?.monogram_gradient) }}
            >
              {user ? userInitials(user) : "?"}
            </div>
          }
          action={
            <Button
              variant="outline"
              size="sm"
              className="w-full"
              disabled={busy !== null}
              onClick={() => setPickingGradient(true)}
            >
              <Palette className="h-3.5 w-3.5" />
              Change Color
            </Button>
          }
        />
      </div>

      <NoticeRow notice={notice} />

      {editing && (
        <AvatarEditorModal file={editing} onClose={() => setEditing(null)} onSaved={() => refresh()} />
      )}
      {pickingGradient && (
        <GradientModal
          current={avatar?.monogram_gradient ?? "brand"}
          onClose={() => setPickingGradient(false)}
          onPick={(slug) =>
            act("gradient", () => api.setMonogramGradient(slug), "Color updated.").then(() =>
              setPickingGradient(false),
            )
          }
        />
      )}
    </Card>
  );
}

/** One third-width option: preview tile (click to select), label, and its action button. */
function AvatarOption({
  label,
  selected,
  selectable,
  unavailableHint,
  badge,
  onSelect,
  preview,
  action,
}: {
  label: string;
  selected: boolean;
  selectable: boolean;
  unavailableHint?: string;
  badge?: string;
  onSelect: () => void;
  preview: ReactNode;
  action: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-2">
      <button
        type="button"
        onClick={() => selectable && !selected && onSelect()}
        disabled={!selectable}
        title={selectable ? undefined : unavailableHint}
        className={
          "relative aspect-square w-full overflow-hidden rounded-xl border-2 transition " +
          (selected
            ? "border-[hsl(var(--brand))] shadow-glow"
            : selectable
              ? "border-border opacity-70 hover:opacity-100"
              : "cursor-not-allowed border-border opacity-40")
        }
      >
        {preview}
        {selected && (
          <span className="absolute right-1.5 top-1.5 grid h-6 w-6 place-items-center rounded-full brand-gradient text-white shadow">
            <Check className="h-3.5 w-3.5" />
          </span>
        )}
        {badge && (
          <span className="absolute bottom-1.5 left-1.5 rounded bg-black/60 px-1.5 py-0.5 text-[10px] text-white">
            {badge}
          </span>
        )}
      </button>
      <div className="text-center text-xs font-medium text-muted">{label}</div>
      {action}
    </div>
  );
}

function EmptyPreview({ icon }: { icon: ReactNode }) {
  return (
    <div className="grid h-full w-full place-items-center bg-surface2 text-muted/40">{icon}</div>
  );
}

/** Gradient choices for the initials monogram — stored server-side, shown to everyone. */
function GradientModal({
  current,
  onClose,
  onPick,
}: {
  current: string;
  onClose: () => void;
  onPick: (slug: string) => void;
}) {
  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/50 p-4" onClick={onClose}>
      <div className="w-full max-w-sm" onClick={(e) => e.stopPropagation()}>
      <Card className="p-6">
        <h3 className="font-display font-semibold">Initials color</h3>
        <p className="mt-1 text-sm text-muted">Pick a gradient — everyone who sees your initials sees it.</p>
        <div className="mt-4 grid grid-cols-4 gap-3">
          {MONOGRAM_GRADIENTS.map((g) => (
            <button
              key={g.slug}
              type="button"
              onClick={() => onPick(g.slug)}
              title={g.label}
              className={
                "flex flex-col items-center gap-1.5 rounded-lg p-2 transition hover:bg-surface2 " +
                (g.slug === current ? "bg-surface2 ring-2 ring-[hsl(var(--brand))]" : "")
              }
            >
              <span className="h-10 w-10 rounded-full" style={{ background: g.css }} />
              <span className="text-[10px] text-muted">{g.label}</span>
            </button>
          ))}
        </div>
        <div className="mt-4 text-right">
          <Button variant="ghost" size="sm" onClick={onClose}>
            Close
          </Button>
        </div>
      </Card>
      </div>
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
  const { user, refresh } = useAuth();
  // SSO-only accounts have no password yet: no "current" to confirm — the form SETS one.
  const hasPassword = user?.has_password ?? true;
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
      await api.changePassword(hasPassword ? current : null, next, logoutOthers);
      setCurrent("");
      setNext("");
      setConfirm("");
      setNotice({
        tone: "ok",
        text: !hasPassword
          ? "Password set — you can now also sign in with your email."
          : logoutOthers
            ? "Password changed. Other devices were signed out."
            : "Password changed.",
      });
      if (!hasPassword) await refresh(); // has_password flips → the form becomes change-mode
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
      {!hasPassword && (
        <p className="mt-2 text-sm text-muted">
          You signed in with Google, so this account has no password yet. Set one to also sign
          in with your email address.
        </p>
      )}
      <form className="mt-4 grid gap-4 sm:grid-cols-3" onSubmit={submit}>
        {hasPassword && (
          <div>
            <Label>Current password</Label>
            <Input type="password" value={current} onChange={(e) => setCurrent(e.target.value)} required />
          </div>
        )}
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
            {busy ? "Saving…" : hasPassword ? "Change password" : "Set password"}
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
