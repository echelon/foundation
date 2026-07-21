/**
 * Google Identity Services (the modern, FedCM-based "Sign in with Google" — the old
 * gapi.auth2 library is dead). The GSI script renders Google's official button; on consent
 * it hands us a **credential** (an ID-token JWT) which the BACKEND verifies — nothing about
 * identity is decided client-side.
 *
 * `VITE_GOOGLE_CLIENT_ID` is the OAuth client id — a public identifier, safe to ship to
 * browsers (this flow involves no client secret anywhere). Unset → Google sign-in hides.
 */

const CLIENT_ID = (import.meta.env.VITE_GOOGLE_CLIENT_ID as string | undefined)?.trim() || undefined;

export const googleSsoEnabled = Boolean(CLIENT_ID);

type CredentialResponse = { credential: string };

/** The slice of the GSI API we use. */
type GsiApi = {
  accounts: {
    id: {
      initialize: (config: { client_id: string; callback: (r: CredentialResponse) => void }) => void;
      renderButton: (el: HTMLElement, options: Record<string, unknown>) => void;
    };
  };
};

declare global {
  interface Window {
    google?: GsiApi;
  }
}

let scriptPromise: Promise<void> | null = null;

/** Inject https://accounts.google.com/gsi/client once; resolves when the API is available. */
function loadGsiScript(): Promise<void> {
  if (window.google?.accounts?.id) return Promise.resolve();
  if (scriptPromise) return scriptPromise;
  scriptPromise = new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = "https://accounts.google.com/gsi/client";
    script.async = true;
    script.onload = () => resolve();
    script.onerror = () => {
      scriptPromise = null; // allow a retry on the next render
      reject(new Error("Couldn't load Google sign-in."));
    };
    document.head.appendChild(script);
  });
  return scriptPromise;
}

/**
 * Render Google's official button into `el`; `onCredential` fires with the ID-token JWT
 * after the user consents. Throws if GSI is unconfigured or the script can't load —
 * callers hide the button and move on (SSO is additive, never load-bearing).
 */
export async function renderGoogleButton(el: HTMLElement, onCredential: (credential: string) => void): Promise<void> {
  if (!CLIENT_ID) throw new Error("VITE_GOOGLE_CLIENT_ID is not set");
  await loadGsiScript();
  const gsi = window.google?.accounts?.id;
  if (!gsi) throw new Error("Google sign-in failed to initialize.");
  gsi.initialize({ client_id: CLIENT_ID, callback: (r) => onCredential(r.credential) });
  gsi.renderButton(el, { theme: "outline", size: "large", text: "continue_with", width: 320 });
}
