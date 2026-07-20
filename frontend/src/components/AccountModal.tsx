import { useState, type FormEvent } from "react";
import { X } from "lucide-react";
import { Button, Input, Label, Modal } from "@/components/ui";
import { ApiError } from "@/lib/api";
import { useAuth } from "@/lib/auth";

/**
 * The account-creation modal popped when an anonymous visitor tries to run/pay in
 * the playground (or anywhere else that needs an account mid-flow). Defaults to
 * signup with a switch to sign-in. On success calls `onAuthed` (e.g. to resume
 * the blocked action) and closes.
 */
export function AccountModal({
  open,
  onClose,
  onAuthed,
  reason,
}: {
  open: boolean;
  onClose: () => void;
  onAuthed?: () => void;
  reason?: string;
}) {
  const { signup, login } = useAuth();
  const [mode, setMode] = useState<"signup" | "login">("signup");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      if (mode === "signup") await signup(email, password, displayName || undefined);
      else await login(email, password);
      onClose();
      onAuthed?.();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Something went wrong — try again.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose}>
      <div className="flex items-start justify-between">
        <div>
          <h2 className="font-display text-xl font-bold tracking-tight">
            {mode === "signup" ? "Create your account" : "Welcome back"}
          </h2>
          <p className="mt-1 text-sm text-muted">
            {reason ?? "You need an account to run generations."}
          </p>
        </div>
        <button onClick={onClose} className="rounded-md p-1 text-muted transition hover:text-fg" aria-label="Close">
          <X className="h-5 w-5" />
        </button>
      </div>

      <form className="mt-5 space-y-4" onSubmit={submit}>
        {mode === "signup" && (
          <div>
            <Label>Name</Label>
            <Input placeholder="Ada Rivera" value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
          </div>
        )}
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
          <Label>Password</Label>
          <Input
            type="password"
            placeholder={mode === "signup" ? "At least 8 characters" : "••••••••"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </div>
        {error && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
        <Button type="submit" className="w-full" disabled={busy}>
          {busy ? "One moment…" : mode === "signup" ? "Create account" : "Sign in"}
        </Button>
      </form>

      <p className="mt-4 text-center text-sm text-muted">
        {mode === "signup" ? (
          <>
            Already have an account?{" "}
            <button className="font-medium text-brand hover:underline" onClick={() => setMode("login")}>
              Sign in
            </button>
          </>
        ) : (
          <>
            New here?{" "}
            <button className="font-medium text-brand hover:underline" onClick={() => setMode("signup")}>
              Create an account
            </button>
          </>
        )}
      </p>
    </Modal>
  );
}
