import { useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { CheckCircle2 } from "lucide-react";
import { Button } from "@/components/ui";
import { api, ApiError } from "@/lib/api";

/** Where the emailed verification link lands: `/verify-email?token=…`. Confirmation is behind
 * a button (not on page load) so email scanners that prefetch links can't consume the token. */
export default function VerifyEmail() {
  const [params] = useSearchParams();
  const token = params.get("token") ?? "";
  const [state, setState] = useState<"idle" | "busy" | "done">("idle");
  const [error, setError] = useState<string | null>(null);

  const confirm = async () => {
    setState("busy");
    setError(null);
    try {
      await api.confirmEmailVerification(token);
      setState("done");
    } catch (err) {
      setState("idle");
      setError(err instanceof ApiError ? err.message : "Something went wrong — try again.");
    }
  };

  return (
    <div className="w-full max-w-sm">
      <div className="rounded-2xl border border-border bg-surface p-7 text-center shadow-glow">
        <h1 className="font-display text-2xl font-bold tracking-tight">Verify your email</h1>
        {state === "done" ? (
          <>
            <div className="mt-4 flex items-start justify-center gap-2 rounded-lg bg-emerald-500/10 p-4 text-sm text-emerald-500">
              <CheckCircle2 className="mt-0.5 h-5 w-5 shrink-0" />
              <p>Your email is verified. Ya ha ha!</p>
            </div>
            <Link to="/dashboard/settings" className="mt-5 inline-block text-sm font-medium text-brand hover:underline">
              Back to settings →
            </Link>
          </>
        ) : !token ? (
          <p className="mt-3 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">
            This link is missing its token — request a new verification email from settings.
          </p>
        ) : (
          <>
            <p className="mt-2 text-sm text-muted">One click to confirm this address belongs to you.</p>
            {error && <p className="mt-3 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
            <Button className="mt-5 w-full" onClick={confirm} disabled={state === "busy"}>
              {state === "busy" ? "Verifying…" : "Confirm my email"}
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
