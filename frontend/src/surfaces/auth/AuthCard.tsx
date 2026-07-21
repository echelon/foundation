import { type ReactNode } from "react";
import { Card } from "@/components/ui";
import { GoogleSignInButton } from "@/components/GoogleSignInButton";
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

/** The SSO options. Google is live (GIS); hidden entirely when unconfigured. `onSignedIn`
 * fires after the session is established (signup or login — same button). */
export function SsoRow({ onSignedIn }: { onSignedIn: () => void }) {
  if (!googleSsoEnabled) return null;
  return <GoogleSignInButton onSuccess={onSignedIn} />;
}

/** Renders nothing when there are no SSO options to divide from. */
export function Divider() {
  if (!googleSsoEnabled) return null;
  return (
    <div className="flex items-center gap-3 text-xs text-muted">
      <span className="h-px flex-1 bg-border" />
      or
      <span className="h-px flex-1 bg-border" />
    </div>
  );
}
