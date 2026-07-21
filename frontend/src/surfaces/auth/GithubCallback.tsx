import { useEffect, useRef, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { api, ApiError } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { consumeGithubFlow } from "@/lib/github";
import { safeRedirectPath } from "@/lib/redirect";
import { AuthCard } from "./AuthCard";

/**
 * Where GitHub redirects back to (`/auth/github/callback?code&state`). Validates `state`
 * against the stashed flow, hands the code (+ PKCE verifier) to the backend for the secret
 * exchange, then continues to wherever the user was headed.
 */
export default function GithubCallback() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const { refresh } = useAuth();
  const [error, setError] = useState<string | null>(null);
  const ran = useRef(false);

  useEffect(() => {
    if (ran.current) return; // StrictMode double-mount guard — the flow stash is single-use
    ran.current = true;

    const ghError = params.get("error");
    if (ghError) {
      setError(ghError === "access_denied" ? "GitHub sign-in was cancelled." : "GitHub reported an error — try again.");
      return;
    }
    const code = params.get("code");
    const flow = consumeGithubFlow(params.get("state"));
    if (!code || !flow) {
      setError("This sign-in attempt is stale or invalid — please start again.");
      return;
    }
    api
      .oauthGithubCallback(code, flow.verifier)
      .then(async () => {
        await refresh();
        navigate(safeRedirectPath(flow.destination) ?? "/dashboard", { replace: true });
      })
      .catch((err) => {
        setError(err instanceof ApiError ? err.message : "GitHub sign-in failed — try again.");
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <AuthCard
      title="Signing in with GitHub"
      subtitle={error ? "That didn't work." : "One moment — completing sign-in…"}
      footer={
        <>
          <Link to="/login" className="font-medium text-brand hover:underline">
            Back to sign in
          </Link>
        </>
      }
    >
      {error && <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}
    </AuthCard>
  );
}
