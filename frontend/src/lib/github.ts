/**
 * GitHub SSO — the web application flow in its modern GitHub-App form, with `state` (CSRF)
 * and PKCE (both strongly recommended by GitHub). We redirect to GitHub's authorize page;
 * GitHub redirects back to `/auth/github/callback` with a `code`; the BACKEND exchanges it
 * (that's where the client secret lives — it never touches the browser).
 *
 * `VITE_GITHUB_CLIENT_ID` is public. Unset → the GitHub button hides.
 */

const CLIENT_ID = (import.meta.env.VITE_GITHUB_CLIENT_ID as string | undefined)?.trim() || undefined;

export const githubSsoEnabled = Boolean(CLIENT_ID);

const FLOW_KEY = "foundation:github-oauth-flow";

function base64url(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** Begin the flow: mint state + PKCE pair, stash them (with the post-login destination and
 * the intent — "login", or "link" to attach GitHub to the CURRENT user from settings) in
 * sessionStorage, and hand the tab to GitHub. */
export async function startGithubSignIn(destination: string, intent: "login" | "link" = "login"): Promise<void> {
  if (!CLIENT_ID) throw new Error("VITE_GITHUB_CLIENT_ID is not set");
  const state = crypto.randomUUID();
  const verifier = base64url(crypto.getRandomValues(new Uint8Array(32)));
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(verifier));
  const challenge = base64url(new Uint8Array(digest));
  sessionStorage.setItem(FLOW_KEY, JSON.stringify({ state, verifier, destination, intent }));

  const params = new URLSearchParams({
    client_id: CLIENT_ID,
    redirect_uri: `${window.location.origin}/auth/github/callback`,
    state,
    code_challenge: challenge,
    code_challenge_method: "S256",
  });
  window.location.assign(`https://github.com/login/oauth/authorize?${params}`);
}

/** Finish the flow: validate GitHub's echoed `state` against the stashed one (single-use).
 * Null = stale/forged/expired — the caller shows "try again". */
export function consumeGithubFlow(
  returnedState: string | null,
): { verifier: string; destination: string; intent: "login" | "link" } | null {
  const raw = sessionStorage.getItem(FLOW_KEY);
  sessionStorage.removeItem(FLOW_KEY); // single-use, success or not
  if (!raw || !returnedState) return null;
  try {
    const flow = JSON.parse(raw) as { state?: string; verifier?: string; destination?: string; intent?: string };
    if (flow.state !== returnedState || !flow.verifier) return null;
    return {
      verifier: flow.verifier,
      destination: flow.destination ?? "/dashboard",
      intent: flow.intent === "link" ? "link" : "login",
    };
  } catch {
    return null;
  }
}
