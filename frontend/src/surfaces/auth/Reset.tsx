import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { CheckCircle2 } from "lucide-react";
import { Button, Input, Label } from "@/components/ui";
import { api, ApiError } from "@/lib/api";
import { AuthCard } from "./AuthCard";

export default function Reset() {
  const [email, setEmail] = useState("");
  const [sent, setSent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.requestPasswordReset(email);
      setSent(true); // the response is identical whether or not the account exists
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong — try again.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <AuthCard
      title="Reset password"
      subtitle={sent ? "Check your inbox for the reset link." : "We'll email you a link to reset your password."}
      footer={
        <Link to="/login" className="font-medium text-brand hover:underline">
          Back to sign in
        </Link>
      }
    >
      {sent ? (
        <div className="flex items-start gap-3 rounded-lg bg-emerald-500/10 p-4 text-sm text-emerald-500">
          <CheckCircle2 className="mt-0.5 h-5 w-5 shrink-0" />
          <p>If an account exists for that email, a reset link is on its way.</p>
        </div>
      ) : (
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
          {error && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
          <Button type="submit" className="w-full" disabled={busy}>
            {busy ? "Sending…" : "Send reset link"}
          </Button>
        </form>
      )}
    </AuthCard>
  );
}
