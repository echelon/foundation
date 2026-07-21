import { useEffect, useRef, useState } from "react";
import { api, ApiError } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { googleSsoEnabled, renderGoogleButton } from "@/lib/google";

/**
 * The real "Continue with Google" button (GIS). On consent, the credential goes to the
 * backend for verification; success establishes the session (signup on first visit, login
 * after). Renders nothing when VITE_GOOGLE_CLIENT_ID is unset — SSO is additive, and its
 * failure modes must never break the email/password forms around it.
 */
export function GoogleSignInButton({
  onSuccess,
  onCredential,
}: {
  onSuccess: () => void;
  /** Override what happens with the credential (e.g. LINK it to the current user from
   * settings) — default is login/signup via the oauth callback. */
  onCredential?: (credential: string) => Promise<void>;
}) {
  const container = useRef<HTMLDivElement>(null);
  const { refresh } = useAuth();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!googleSsoEnabled || !container.current) return;
    let cancelled = false;
    renderGoogleButton(container.current, (credential) => {
      if (cancelled) return;
      setBusy(true);
      setError(null);
      const complete = onCredential
        ? onCredential(credential)
        : api.oauthCallback("google", credential).then(() => undefined);
      complete
        .then(async () => {
          await refresh(); // /me → user (with avatar) into the auth context
          if (!cancelled) onSuccess();
        })
        .catch((err) => {
          if (!cancelled) setError(err instanceof ApiError ? err.message : "Google sign-in failed — try again.");
        })
        .finally(() => {
          if (!cancelled) setBusy(false);
        });
    }).catch(() => {
      // Script blocked / offline: quietly show nothing rather than a dead button.
      if (!cancelled) setError(null);
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!googleSsoEnabled) return null;

  return (
    <div>
      <div ref={container} className={busy ? "pointer-events-none opacity-60" : undefined} />
      {error && <p className="mt-2 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
    </div>
  );
}
