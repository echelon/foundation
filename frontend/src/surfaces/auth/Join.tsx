import { useEffect, useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { Button, Input, Label } from "@/components/ui";
import { api, ApiError, type ApiInvitation } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { rememberLastAccount } from "@/lib/org";
import { AuthCard } from "./AuthCard";

/**
 * The join page (accounts.md §4). Entry: the emailed magic link (`/join?token=…`), the
 * console's copy link, or a hand-typed short code (`/join` + `7F3K-QT9M`). Resolves the
 * invitation, then branches on the visitor's state:
 *
 *   1. signed out, no user for the invite email → signup form, email LOCKED to the invite's;
 *   2. signed out, a user exists for that email  → login form, then accept;
 *   3. signed in → "join {account} as {role}?" confirmation (mismatched email allowed —
 *      membership binds to the authenticated user; the server records the mismatch).
 */
export default function Join() {
  const [params] = useSearchParams();
  const { user, loading: authLoading } = useAuth();

  // The code/token being redeemed — from the URL, or typed into the code form.
  const fromUrl = params.get("token") ?? params.get("code");
  const [credential, setCredential] = useState<string | null>(fromUrl);
  const [codeInput, setCodeInput] = useState("");

  const [invite, setInvite] = useState<ApiInvitation | null>(null);
  const [resolving, setResolving] = useState(Boolean(fromUrl));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!credential) return;
    let cancelled = false;
    setResolving(true);
    setError(null);
    api
      .resolveInvitation(credential)
      .then((inv) => {
        if (!cancelled) setInvite(inv);
      })
      .catch((err) => {
        if (cancelled) return;
        setInvite(null);
        setError(err instanceof ApiError ? err.message : "Couldn't look up that invitation.");
      })
      .finally(() => {
        if (!cancelled) setResolving(false);
      });
    return () => {
      cancelled = true;
    };
  }, [credential]);

  // Wait for the /me probe so a signed-in visitor doesn't flash the signup form.
  if (resolving || authLoading) {
    return (
      <AuthCard title="Join an organization" subtitle="Looking up your invitation…" footer={null}>
        <p className="text-sm text-muted">One moment.</p>
      </AuthCard>
    );
  }

  if (!invite) {
    return (
      <AuthCard
        title="Join an organization"
        subtitle="Enter the invitation code you were given (it looks like 7F3K-QT9M)."
        footer={
          <>
            No invitation?{" "}
            <Link to="/signup" className="font-medium text-brand hover:underline">
              Create your own account
            </Link>
          </>
        }
      >
        <form
          className="space-y-4"
          onSubmit={(e: FormEvent) => {
            e.preventDefault();
            if (codeInput.trim()) setCredential(codeInput.trim());
          }}
        >
          <div>
            <Label>Invitation code</Label>
            <Input
              placeholder="7F3K-QT9M"
              value={codeInput}
              onChange={(e) => setCodeInput(e.target.value)}
              className="font-mono uppercase"
              required
            />
          </div>
          {error && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
          <Button type="submit" className="w-full">
            Find invitation
          </Button>
        </form>
      </AuthCard>
    );
  }

  const orgName = invite.account_name ?? "the organization";
  if (user) return <ConfirmJoin invite={invite} orgName={orgName} sessionEmail={user.email} />;
  if (invite.invitee_user_exists) return <LoginAndJoin invite={invite} orgName={orgName} />;
  return <SignupAndJoin invite={invite} orgName={orgName} credential={credential!} />;
}

/** State 3: signed in — confirm and accept. */
function ConfirmJoin({ invite, orgName, sessionEmail }: { invite: ApiInvitation; orgName: string; sessionEmail: string }) {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const mismatch = sessionEmail.toLowerCase() !== invite.email.toLowerCase();

  const accept = async () => {
    setBusy(true);
    setErr(null);
    try {
      await api.acceptInvitation(invite.invitation_id);
      rememberLastAccount(invite.account_id); // scope the dashboard to the org just joined
      navigate("/dashboard/organization", { replace: true });
    } catch (e) {
      setErr(e instanceof ApiError ? e.message : "Couldn't accept the invitation — try again.");
      setBusy(false);
    }
  };

  return (
    <AuthCard
      title={`Join ${orgName}?`}
      subtitle={`You've been invited as ${invite.role}.`}
      footer={
        <>
          Wrong account?{" "}
          <Link to="/login" className="font-medium text-brand hover:underline">
            Switch user
          </Link>
        </>
      }
    >
      {mismatch && (
        <p className="rounded-lg bg-amber-500/10 px-3 py-2 text-sm text-amber-600">
          This invitation was sent to <strong>{invite.email}</strong>, but you're signed in as{" "}
          <strong>{sessionEmail}</strong>. Joining will add <strong>{sessionEmail}</strong> to {orgName}.
        </p>
      )}
      {err && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{err}</p>}
      <Button className="w-full" onClick={() => void accept()} disabled={busy}>
        {busy ? "Joining…" : `Join as ${invite.role}`}
      </Button>
    </AuthCard>
  );
}

/** State 2: signed out but a user exists for the invited email — log in, then accept. */
function LoginAndJoin({ invite, orgName }: { invite: ApiInvitation; orgName: string }) {
  const navigate = useNavigate();
  const { login } = useAuth();
  const [email, setEmail] = useState(invite.email);
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setErr(null);
    try {
      await login(email, password);
      await api.acceptInvitation(invite.invitation_id);
      rememberLastAccount(invite.account_id); // scope the dashboard to the org just joined
      navigate("/dashboard/organization", { replace: true });
    } catch (e2) {
      setErr(e2 instanceof ApiError ? e2.message : "Something went wrong — try again.");
      setBusy(false);
    }
  };

  return (
    <AuthCard
      title={`Join ${orgName}`}
      subtitle={`You already have an account — sign in to join as ${invite.role}.`}
      footer={
        <>
          Forgot your password?{" "}
          <Link to="/reset" className="font-medium text-brand hover:underline">
            Reset it
          </Link>
        </>
      }
    >
      <form className="space-y-4" onSubmit={submit}>
        <div>
          <Label>Email</Label>
          <Input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
        </div>
        <div>
          <Label>Password</Label>
          <Input type="password" placeholder="••••••••" value={password} onChange={(e) => setPassword(e.target.value)} required />
        </div>
        {err && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{err}</p>}
        <Button type="submit" className="w-full" disabled={busy}>
          {busy ? "Signing in…" : "Sign in and join"}
        </Button>
      </form>
    </AuthCard>
  );
}

/** State 1: signed out, no user for the invited email — locked-email signup that also joins. */
function SignupAndJoin({ invite, orgName, credential }: { invite: ApiInvitation; orgName: string; credential: string }) {
  const navigate = useNavigate();
  const { signup } = useAuth();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setErr(null);
    try {
      // The backend accepts the invitation inside the signup transaction.
      await signup(invite.email, password, name || undefined, credential);
      rememberLastAccount(invite.account_id); // scope the dashboard to the org just joined
      navigate("/dashboard/organization", { replace: true });
    } catch (e2) {
      setErr(e2 instanceof ApiError ? e2.message : "Something went wrong — try again.");
      setBusy(false);
    }
  };

  return (
    <AuthCard
      title={`Join ${orgName}`}
      subtitle={`Create your account to join as ${invite.role}.`}
      footer={
        <>
          Already have an account?{" "}
          <Link to="/login" className="font-medium text-brand hover:underline">
            Sign in
          </Link>
        </>
      }
    >
      <form className="space-y-4" onSubmit={submit}>
        <div>
          <Label>Email</Label>
          {/* Locked to the invitation's email (accounts.md §4 flow 1) — the API enforces it. */}
          <Input type="email" value={invite.email} disabled readOnly />
        </div>
        <div>
          <Label>Full name</Label>
          <Input placeholder="Ada Rivera" value={name} onChange={(e) => setName(e.target.value)} />
        </div>
        <div>
          <Label>Password</Label>
          <Input
            type="password"
            placeholder="At least 8 characters"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </div>
        {err && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{err}</p>}
        <Button type="submit" className="w-full" disabled={busy}>
          {busy ? "Joining…" : "Create account and join"}
        </Button>
      </form>
    </AuthCard>
  );
}
