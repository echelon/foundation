import { useState, type FormEvent } from "react";
import { Link, useLocation, useNavigate, useSearchParams } from "react-router-dom";
import { Button, Input, Label } from "@/components/ui";
import { ApiError } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { safeRedirectPath } from "@/lib/redirect";
import { AuthCard, Divider, SsoRow } from "./AuthCard";

export default function Login() {
  const navigate = useNavigate();
  const location = useLocation();
  const [params] = useSearchParams();
  const { login } = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Post-login destination: a safe same-origin ?redirect_to, else where RequireAuth
  // bounced them from, else the dashboard.
  const from =
    safeRedirectPath(params.get("redirect_to")) ??
    (location.state as { from?: string } | null)?.from ??
    "/dashboard";

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await login(email, password);
      navigate(from, { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong — try again.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <AuthCard
      title="Welcome back"
      subtitle="Sign in to your Foundation account."
      footer={
        <>
          New here?{" "}
          <Link to={`/signup${location.search}`} className="font-medium text-brand hover:underline">
            Create an account
          </Link>
        </>
      }
    >
      <SsoRow onSignedIn={() => navigate(from, { replace: true })} />
      <Divider />
      <form className="space-y-4" onSubmit={submit}>
        <div>
          <Label>Email</Label>
          <Input
            type="email"
            placeholder="you@company.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
        </div>
        <div>
          <Label hint="">Password</Label>
          <Input
            type="password"
            placeholder="••••••••"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
          <div className="mt-1.5 text-right">
            <Link to="/reset" className="text-xs text-muted hover:text-fg">
              Forgot password?
            </Link>
          </div>
        </div>
        {error && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
        <Button type="submit" className="w-full" disabled={busy}>
          {busy ? "Signing in…" : "Sign in"}
        </Button>
      </form>
    </AuthCard>
  );
}
