import { useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { Button, Input } from "@/components/ui";
import { ApiError } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { demoEnabled } from "@/lib/gateway";
import { defaultSignupDestination, isConnectDestination } from "@/lib/onboarding";
import { safeRedirectPath } from "@/lib/redirect";
import { AuthCard, Divider, SsoRow } from "./AuthCard";

export default function Signup() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const { signup } = useAuth();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestedDestination = safeRedirectPath(params.get("redirect_to"));
  const destination = requestedDestination ?? defaultSignupDestination(demoEnabled);
  const connecting = isConnectDestination(destination);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await signup(email, password, name || undefined);
      navigate(destination, { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong — try again.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <AuthCard
      title="Create your account"
      subtitle={connecting ? "Create an account to get your API key." : "Start generating in minutes. No card required to explore."}
      footer={
        <>
          Already have an account?{" "}
          <Link to={`/login${params.toString() ? `?${params}` : ""}`} className="font-medium text-brand hover:underline">
            Sign in
          </Link>
        </>
      }
    >
      <SsoRow
        onSignedIn={() => navigate(destination, { replace: true })}
        destination={destination}
      />
      <Divider />
      <form className="space-y-4" onSubmit={submit}>
        {(!requestedDestination || !connecting) && <label className="block space-y-1.5 text-sm font-medium">
          <span>Full name</span>
          <Input placeholder="Ada Rivera" autoComplete="name" value={name} onChange={(e) => setName(e.target.value)} />
        </label>}
        <label className="block space-y-1.5 text-sm font-medium">
          <span>Email</span>
          <Input
            type="email"
            placeholder="you@company.com"
            autoComplete="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
        </label>
        <label className="block space-y-1.5 text-sm font-medium">
          <span>Password</span>
          <Input
            type="password"
            placeholder="At least 8 characters"
            autoComplete="new-password"
            minLength={8}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </label>
        {error && <p role="alert" className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
        <Button type="submit" className="w-full" disabled={busy}>
          {busy ? "Creating account…" : "Create account"}
        </Button>
      </form>
    </AuthCard>
  );
}
