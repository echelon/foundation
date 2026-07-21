import { useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { Button, Input, Label } from "@/components/ui";
import { ApiError } from "@/lib/api";
import { useAuth } from "@/lib/auth";
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

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await signup(email, password, name || undefined);
      // A safe same-origin ?redirect_to wins; else the dashboard.
      navigate(safeRedirectPath(params.get("redirect_to")) ?? "/dashboard", { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong — try again.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <AuthCard
      title="Create your account"
      subtitle="Start generating in minutes. No card required to explore."
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
        onSignedIn={() => navigate(safeRedirectPath(params.get("redirect_to")) ?? "/dashboard", { replace: true })}
        destination={safeRedirectPath(params.get("redirect_to")) ?? "/dashboard"}
      />
      <Divider />
      <form className="space-y-4" onSubmit={submit}>
        <div>
          <Label>Full name</Label>
          <Input placeholder="Ada Rivera" value={name} onChange={(e) => setName(e.target.value)} />
        </div>
        <div>
          <Label>Work email</Label>
          <Input
            type="email"
            placeholder="you@company.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
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
        {error && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
        <Button type="submit" className="w-full" disabled={busy}>
          {busy ? "Creating account…" : "Create account"}
        </Button>
        <p className="text-center text-xs text-muted">
          By continuing you agree to the{" "}
          <a href="#" className="text-muted underline hover:text-fg">
            Terms
          </a>{" "}
          and{" "}
          <a href="#" className="text-muted underline hover:text-fg">
            Privacy Policy
          </a>
          .
        </p>
      </form>
    </AuthCard>
  );
}
