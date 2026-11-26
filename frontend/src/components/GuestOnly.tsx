import { Navigate, Outlet, useLocation, useMatch, useSearchParams } from "react-router-dom";
import { Loader2 } from "lucide-react";
import { useAuth } from "@/lib/auth";
import { demoEnabled } from "@/lib/gateway";
import { defaultSignupDestination } from "@/lib/onboarding";
import { safeRedirectPath } from "@/lib/redirect";

/**
 * The inverse of [`RequireAuth`]: sessionless pages (login, signup, reset request) render
 * only for signed-out visitors. A logged-in user is bounced to `?redirect_to={path}` when
 * present and same-origin-safe ([`safeRedirectPath`]), else back to where RequireAuth
 * bounced them from, else the default destination for the current form.
 */
export function GuestOnly() {
  const { user, loading } = useAuth();
  const location = useLocation();
  const signup = useMatch("/signup");
  const [params] = useSearchParams();

  if (loading) {
    return (
      <div className="grid min-h-screen place-items-center bg-bg">
        <Loader2 className="h-6 w-6 animate-spin text-muted" />
      </div>
    );
  }
  if (user) {
    const from = (location.state as { from?: string } | null)?.from;
    const fallback = signup ? defaultSignupDestination(demoEnabled) : "/dashboard";
    return <Navigate to={safeRedirectPath(params.get("redirect_to")) ?? from ?? fallback} replace />;
  }
  return <Outlet />;
}
