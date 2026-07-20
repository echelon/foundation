import { useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { Button, Input, Label } from "@/components/ui";
import { api, ApiError } from "@/lib/api";
import { AuthCard } from "./AuthCard";

/** Where the emailed reset link lands: `/reset-password?token=…`. */
export default function ResetConfirm() {
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const token = params.get("token") ?? "";
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [logoutOthers, setLogoutOthers] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (password !== confirm) {
      setError("Passwords don't match.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await api.confirmPasswordReset(token, password, logoutOthers);
      navigate("/login", { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong — try again.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <AuthCard
      title="Choose a new password"
      subtitle="Almost done — set the new password for your account."
      footer={
        <Link to="/login" className="font-medium text-brand hover:underline">
          Back to sign in
        </Link>
      }
    >
      {!token ? (
        <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">
          This reset link is missing its token. Request a new one from the{" "}
          <Link to="/reset" className="underline">
            reset page
          </Link>
          .
        </p>
      ) : (
        <form className="space-y-4" onSubmit={submit}>
          <div>
            <Label>New password</Label>
            <Input
              type="password"
              placeholder="At least 8 characters"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </div>
          <div>
            <Label>Confirm new password</Label>
            <Input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} required />
          </div>
          <label className="flex items-center gap-2 text-sm text-muted">
            <input
              type="checkbox"
              checked={logoutOthers}
              onChange={(e) => setLogoutOthers(e.target.checked)}
              className="h-4 w-4 accent-[hsl(var(--brand))]"
            />
            Log out my other devices
          </label>
          {error && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
          <Button type="submit" className="w-full" disabled={busy}>
            {busy ? "Saving…" : "Set new password"}
          </Button>
        </form>
      )}
    </AuthCard>
  );
}
