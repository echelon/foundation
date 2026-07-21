import { type ReactNode } from "react";
import { Card } from "@/components/ui";
import { GoogleSignInButton } from "@/components/GoogleSignInButton";
import { githubSsoEnabled, startGithubSignIn } from "@/lib/github";
import { googleSsoEnabled } from "@/lib/google";

/** Shared shell for the auth forms. */
export function AuthCard({ title, subtitle, children, footer }: { title: string; subtitle: string; children: ReactNode; footer: ReactNode }) {
  return (
    <div className="w-full max-w-sm">
      <Card className="p-7 shadow-glow">
        <h1 className="font-display text-2xl font-bold tracking-tight">{title}</h1>
        <p className="mt-1 text-sm text-muted">{subtitle}</p>
        <div className="mt-6 space-y-4">{children}</div>
      </Card>
      <p className="mt-5 text-center text-sm text-muted">{footer}</p>
    </div>
  );
}

/** The SSO options — each hides when unconfigured. `onSignedIn` fires after Google
 * establishes the session in-page; GitHub leaves the page entirely, so it stashes
 * `destination` and the callback route finishes the journey. */
export function SsoRow({ onSignedIn, destination }: { onSignedIn: () => void; destination: string }) {
  if (!googleSsoEnabled && !githubSsoEnabled) return null;
  return (
    <div className="space-y-2">
      {googleSsoEnabled && <GoogleSignInButton onSuccess={onSignedIn} />}
      {githubSsoEnabled && (
        <button
          type="button"
          onClick={() => void startGithubSignIn(destination)}
          className="flex h-10 w-full items-center justify-center gap-2 rounded-lg border border-border bg-surface text-sm font-medium text-fg transition hover:border-brand/50"
        >
          <GithubMark className="h-4 w-4" />
          Continue with GitHub
        </button>
      )}
    </div>
  );
}

/** The Octocat mark (inline so no external asset is involved). */
function GithubMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden className={className}>
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}

/** Renders nothing when there are no SSO options to divide from. */
export function Divider() {
  if (!googleSsoEnabled && !githubSsoEnabled) return null;
  return (
    <div className="flex items-center gap-3 text-xs text-muted">
      <span className="h-px flex-1 bg-border" />
      or
      <span className="h-px flex-1 bg-border" />
    </div>
  );
}
